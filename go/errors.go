package pdfrost

import "errors"

// errInvalidJSON is returned when document is not JSON.
var errInvalidJSON = errors.New("pdfrost: document is not JSON")
