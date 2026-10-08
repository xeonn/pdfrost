// Command server accepts a JRXML template and returns a PDF download.
//
//	POST /render
//	Content-Type: application/xml
//	Body: the .jrxml file
//
// An optional JSON document fills $F fields. Send it as multipart field "data"
// next to file field "template", or as a JSON body:
//
//	{"template": "<jrxml…>", "data": {"docno": "1001"}}
//
// Without data the document is {}. The response is application/pdf with
// Content-Disposition: attachment. Unsupported tags are repeated in
// X-Pdfrost-Warning and do not fail the request.
//
//	cd examples/go-server
//	go run .
//	curl -sS -D - -o report.pdf --data-binary @../../jrxmls/Sample.jrxml \
//	  -H 'Content-Type: application/xml' http://127.0.0.1:8080/render
package main

import (
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"log"
	"mime"
	"net/http"
	"path/filepath"
	"strings"
	"time"

	"github.com/xeonn/pdfrost/go"
)

const maxBody = 8 << 20

func main() {
	addr := flag.String("addr", ":8080", "listen address")
	flag.Parse()

	mux := http.NewServeMux()
	mux.HandleFunc("POST /render", renderReport)
	log.Printf("listening on %s", *addr)
	log.Fatal(http.ListenAndServe(*addr, mux))
}

func renderReport(w http.ResponseWriter, r *http.Request) {
	templateXML, document, filename, err := readReport(w, r)
	if err != nil {
		status := http.StatusBadRequest
		var tooLarge *http.MaxBytesError
		if errors.As(err, &tooLarge) {
			status = http.StatusRequestEntityTooLarge
		}
		http.Error(w, err.Error(), status)
		return
	}

	result, err := pdfrost.Render(templateXML, document, time.Now())
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	for _, warning := range result.Warnings {
		w.Header().Add("X-Pdfrost-Warning", warning.Message)
	}
	w.Header().Set("Content-Type", "application/pdf")
	w.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=%q", filename))
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write(result.PDF)
}

func readReport(w http.ResponseWriter, r *http.Request) (templateXML string, document []byte, filename string, err error) {
	r.Body = http.MaxBytesReader(w, r.Body, maxBody)
	mediaType, _, _ := mime.ParseMediaType(r.Header.Get("Content-Type"))
	switch {
	case strings.HasPrefix(mediaType, "multipart/"):
		return readMultipart(r)
	case mediaType == "application/json":
		return readJSON(r)
	default:
		body, err := io.ReadAll(r.Body)
		if err != nil {
			return "", nil, "", err
		}
		if len(strings.TrimSpace(string(body))) == 0 {
			return "", nil, "", errors.New("request body is empty; post the .jrxml file")
		}
		return string(body), []byte("{}"), "report.pdf", nil
	}
}

func readMultipart(r *http.Request) (string, []byte, string, error) {
	if err := r.ParseMultipartForm(maxBody); err != nil {
		return "", nil, "", err
	}
	file, header, err := r.FormFile("template")
	if err != nil {
		return "", nil, "", errors.New("multipart field \"template\" is required")
	}
	defer file.Close()
	body, err := io.ReadAll(io.LimitReader(file, maxBody))
	if err != nil {
		return "", nil, "", err
	}
	document, err := jsonObject(r.FormValue("data"))
	if err != nil {
		return "", nil, "", err
	}
	return string(body), document, pdfName(header.Filename), nil
}

func readJSON(r *http.Request) (string, []byte, string, error) {
	body, err := io.ReadAll(r.Body)
	if err != nil {
		return "", nil, "", err
	}
	var payload struct {
		Template string          `json:"template"`
		Data     json.RawMessage `json:"data"`
	}
	if err := json.Unmarshal(body, &payload); err != nil {
		return "", nil, "", errors.New("body must be a JSON object with template")
	}
	if strings.TrimSpace(payload.Template) == "" {
		return "", nil, "", errors.New("template is empty")
	}
	document, err := jsonObject(string(payload.Data))
	if err != nil {
		return "", nil, "", err
	}
	return payload.Template, document, "report.pdf", nil
}

func jsonObject(raw string) ([]byte, error) {
	trimmed := strings.TrimSpace(raw)
	if trimmed == "" || trimmed == "null" {
		return []byte("{}"), nil
	}
	if !json.Valid([]byte(trimmed)) || trimmed[0] != '{' {
		return nil, errors.New("data must be a JSON object")
	}
	return []byte(trimmed), nil
}

func pdfName(name string) string {
	base := filepath.Base(strings.TrimSpace(name))
	if base == "." || base == string(filepath.Separator) || base == "" {
		return "report.pdf"
	}
	stem := strings.TrimSuffix(base, filepath.Ext(base))
	stem = strings.Map(func(r rune) rune {
		switch r {
		case '"', '\\', '\r', '\n':
			return -1
		default:
			return r
		}
	}, stem)
	if stem == "" {
		return "report.pdf"
	}
	return stem + ".pdf"
}
