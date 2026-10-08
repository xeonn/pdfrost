package pdfrost

import (
	"errors"
	"testing"
	"time"
)

func TestRenderRejectsInvalidJSON(t *testing.T) {
	_, err := Render("<jasperReport/>", []byte("not-json"), time.Unix(0, 0))
	if !errors.Is(err, errInvalidJSON) {
		t.Fatalf("got %v, want invalid JSON", err)
	}
}
