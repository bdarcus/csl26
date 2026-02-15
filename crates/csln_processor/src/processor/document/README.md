# Document Processor

This module provides the infrastructure for document-level citation processing. It allows the CSLN processor to scan entire documents (e.g., Djot, Markdown, LaTeX), identify citation markers, and replace them with rendered citations.

## Architecture

The system is designed around the `CitationParser` trait, allowing for format-specific parsing logic while sharing the core processing workflow.

### Core Trait: `CitationParser`

Any new document format must implement the `CitationParser` trait:

```rust
pub trait CitationParser {
    /// Find and extract citations from a document string.
    /// Returns a list of (start_index, end_index, citation_model) tuples.
    fn parse_citations(&self, content: &str) -> Vec<(usize, usize, Citation)>;
}
```

- **Input**: The raw document content as a string.
- **Output**: A list of tuples containing the start index, end index, and the parsed `Citation` model.

## Adding a New Format

To add support for a new document format (e.g., Markdown):

1.  **Create a new file**: `src/processor/document/markdown.rs`.
2.  **Implement the parser**: Use a parsing library (like `winnow` or `pulldown-cmark`) to identify citation markers.
3.  **Register the module**: Add `pub mod markdown;` to `src/processor/document/mod.rs`.
4.  **Update `DocumentFormat`**: Add your format to the `DocumentFormat` enum in `mod.rs`.
5.  **Update `Processor::process_document`**: If your format requires specific post-processing (like Djot's HTML conversion), update the `match` statement in the `process_document` method.

## Existing Implementations

- **Djot (`djot.rs`)**: Uses the `winnow` parser combinator library to identify Djot-style citations (e.g., `[@key]`, `@key[locator]`). It also includes support for converting the final document to HTML using `jotdown`.

## Workflow

The `Processor::process_document` method follows these steps:
1.  Scan the document using the provided `CitationParser`.
2.  Render each identified citation using the configured CSLN style (defaulting to plain text).
3.  Replace the markers with the rendered text.
4.  Append the generated bibliography at the end of the document.
5.  (Optional) Perform final document conversion (e.g., Djot to HTML).
