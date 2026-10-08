// Command render is a small Go program that uses the pdfrost wrapper.
// It reads a JRXML template and one JSON document, calls pdfrost.Render,
// and writes the PDF.
package main

import (
	"flag"
	"fmt"
	"os"
	"time"

	"github.com/xeonn/pdfrost/go"
)

func main() {
	templatePath := flag.String("template", "../../jrxmls/Sample.jrxml", "path to a .jrxml file")
	dataPath := flag.String("data", "data.json", "path to one JSON object")
	outPath := flag.String("out", "sample.pdf", "where to write the PDF")
	flag.Parse()

	if err := run(*templatePath, *dataPath, *outPath); err != nil {
		fmt.Fprintf(os.Stderr, "render: %v\n", err)
		os.Exit(1)
	}
}

func run(templatePath, dataPath, outPath string) error {
	template, err := os.ReadFile(templatePath)
	if err != nil {
		return err
	}
	document, err := os.ReadFile(dataPath)
	if err != nil {
		return err
	}

	result, err := pdfrost.Render(string(template), document, time.Now())
	if err != nil {
		return err
	}

	if err := os.WriteFile(outPath, result.PDF, 0o644); err != nil {
		return err
	}
	fmt.Printf("wrote %s (%d bytes)\n", outPath, len(result.PDF))
	for _, warning := range result.Warnings {
		fmt.Printf("warning: %s\n", warning.Message)
	}
	return nil
}
