// Package pdfrost renders a JRXML template to PDF by calling the Rust engine.
// Parsing and layout are not implemented in Go.
package pdfrost

import (
	"encoding/json"
	"time"
)

// Warning is one unsupported tag, unsupported expression, or missing field.
type Warning struct {
	Message string
}

// Result is a successful render. Warnings may be empty.
// A non-nil error from Render means there is no PDF.
type Result struct {
	PDF      []byte
	Warnings []Warning
}

// Render fills templateXML with one JSON object and returns PDF bytes.
// renderedAt is the instant printed for new java.util.Date(), in UTC.
// document must be a JSON object. The engine does not run the template query.
func Render(templateXML string, document []byte, renderedAt time.Time) (*Result, error) {
	if !json.Valid(document) {
		return nil, errInvalidJSON
	}
	return renderNative(templateXML, document, renderedAt.UTC())
}
