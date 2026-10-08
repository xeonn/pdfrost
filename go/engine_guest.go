package pdfrost

import (
	"context"
	"encoding/binary"
	"fmt"
	"sync"
	"time"

	_ "embed"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
)

//go:embed engine.wasm
var engineWasm []byte

// guest is one WebAssembly instance of the engine. Calls are serialized
// because the module is single-threaded.
type guest struct {
	runtime wazero.Runtime
	alloc   api.Function
	dealloc api.Function
	render  api.Function
	memory  api.Memory
}

var (
	guestOnce sync.Once
	guestInst *guest
	guestErr  error
	guestMu   sync.Mutex
)

func renderNative(template string, document []byte, renderedAt time.Time) (*Result, error) {
	g, err := loadGuest()
	if err != nil {
		return nil, err
	}
	guestMu.Lock()
	defer guestMu.Unlock()
	ctx := context.Background()

	templatePtr, err := g.write(ctx, []byte(template))
	if err != nil {
		return nil, err
	}
	defer g.free(ctx, templatePtr, len(template))

	documentPtr, err := g.write(ctx, document)
	if err != nil {
		return nil, err
	}
	defer g.free(ctx, documentPtr, len(document))

	out, err := g.render.Call(
		ctx,
		uint64(templatePtr),
		uint64(len(template)),
		uint64(documentPtr),
		uint64(len(document)),
		uint64(renderedAt.Unix()),
	)
	if err != nil {
		return nil, fmt.Errorf("pdfrost: wasm render: %w", err)
	}
	if len(out) != 1 {
		return nil, fmt.Errorf("pdfrost: wasm render returned %d values", len(out))
	}
	ptr := uint32(out[0])
	size := uint32(out[0] >> 32)
	if ptr == 0 || size == 0 {
		return nil, fmt.Errorf("pdfrost: wasm render returned an empty result")
	}
	defer g.free(ctx, ptr, int(size))

	buf, ok := g.memory.Read(ptr, size)
	if !ok {
		return nil, fmt.Errorf("pdfrost: wasm result is outside memory")
	}
	copied := append([]byte(nil), buf...)
	return decodeResult(copied)
}

func loadGuest() (*guest, error) {
	guestOnce.Do(func() {
		ctx := context.Background()
		runtime := wazero.NewRuntime(ctx)
		compiled, err := runtime.CompileModule(ctx, engineWasm)
		if err != nil {
			guestErr = fmt.Errorf("pdfrost: compile engine.wasm: %w", err)
			return
		}
		module, err := runtime.InstantiateModule(ctx, compiled, wazero.NewModuleConfig())
		if err != nil {
			guestErr = fmt.Errorf("pdfrost: instantiate engine.wasm: %w", err)
			return
		}
		guestInst = &guest{
			runtime: runtime,
			alloc:   module.ExportedFunction("alloc_bytes"),
			dealloc: module.ExportedFunction("dealloc_bytes"),
			render:  module.ExportedFunction("render"),
			memory:  module.Memory(),
		}
		if guestInst.alloc == nil || guestInst.dealloc == nil || guestInst.render == nil || guestInst.memory == nil {
			guestErr = fmt.Errorf("pdfrost: engine.wasm is missing alloc_bytes, dealloc_bytes, or render")
			guestInst = nil
		}
	})
	return guestInst, guestErr
}

func (g *guest) write(ctx context.Context, data []byte) (uint32, error) {
	if len(data) == 0 {
		return 0, nil
	}
	out, err := g.alloc.Call(ctx, uint64(len(data)))
	if err != nil {
		return 0, fmt.Errorf("pdfrost: wasm alloc: %w", err)
	}
	ptr := uint32(out[0])
	if ptr == 0 {
		return 0, fmt.Errorf("pdfrost: wasm alloc returned a null pointer")
	}
	if !g.memory.Write(ptr, data) {
		return 0, fmt.Errorf("pdfrost: wasm memory write failed")
	}
	return ptr, nil
}

func (g *guest) free(ctx context.Context, ptr uint32, size int) {
	if ptr == 0 || size <= 0 {
		return
	}
	_, _ = g.dealloc.Call(ctx, uint64(ptr), uint64(size))
}

func decodeResult(buf []byte) (*Result, error) {
	if len(buf) < 4 {
		return nil, fmt.Errorf("pdfrost: short wasm result")
	}
	errLen := binary.LittleEndian.Uint32(buf[:4])
	buf = buf[4:]
	if errLen > 0 {
		if uint32(len(buf)) < errLen {
			return nil, fmt.Errorf("pdfrost: truncated wasm error")
		}
		return nil, fmt.Errorf("pdfrost: %s", buf[:errLen])
	}
	pdf, buf, err := takeBytes(buf)
	if err != nil {
		return nil, err
	}
	if len(buf) < 4 {
		return nil, fmt.Errorf("pdfrost: wasm result is missing warnings")
	}
	count := binary.LittleEndian.Uint32(buf[:4])
	buf = buf[4:]
	warnings := make([]Warning, 0, count)
	for i := uint32(0); i < count; i++ {
		var message []byte
		message, buf, err = takeBytes(buf)
		if err != nil {
			return nil, err
		}
		warnings = append(warnings, Warning{Message: string(message)})
	}
	return &Result{PDF: pdf, Warnings: warnings}, nil
}

func takeBytes(buf []byte) ([]byte, []byte, error) {
	if len(buf) < 4 {
		return nil, nil, fmt.Errorf("pdfrost: truncated wasm result")
	}
	n := binary.LittleEndian.Uint32(buf[:4])
	buf = buf[4:]
	if uint32(len(buf)) < n {
		return nil, nil, fmt.Errorf("pdfrost: truncated wasm result")
	}
	return append([]byte(nil), buf[:n]...), buf[n:], nil
}
