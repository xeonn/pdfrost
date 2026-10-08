package pdfrost

import (
	"bytes"
	"os"
	"strings"
	"testing"
	"time"
)

func TestRenderSampleAgreesWithTheEngine(t *testing.T) {
	template, err := os.ReadFile("../jrxmls/Sample.jrxml")
	if err != nil {
		t.Fatal(err)
	}
	document, err := os.ReadFile("../jrxmls/fixtures/sample.json")
	if err != nil {
		t.Fatal(err)
	}
	result, err := Render(string(template), document, time.Unix(1_759_824_000, 0).UTC())
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.HasPrefix(result.PDF, []byte("%PDF")) {
		t.Fatalf("pdf does not start with %%PDF (%d bytes)", len(result.PDF))
	}
	for _, warning := range result.Warnings {
		if strings.Contains(warning.Message, "chart") {
			t.Fatalf("area chart should be plotted, got warning %+v", warning)
		}
	}
}
