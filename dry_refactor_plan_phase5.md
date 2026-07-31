# DRY Refactor Plan: Phase 5 (CLI Output & Helper Refactoring)

This document details identified duplicate patterns in the CLI client and provides a concrete refactoring plan to enforce DRY.

---

## 1. Identified Duplication

### A. Repetitive CLI Table Formatting & String Truncating
* **Location**: `youtube-client/src/main.rs`
* **Problem**: Both `Commands::Subscriptions` and `Commands::Videos` contain manual calculations of column widths, manual repeat calls (`"-".repeat(...)`), indexing, and printing formatting strings.
* **Refactor Plan**:
  * Implement a helper in `youtube-client/src/main.rs` to render formatted table outputs:
    ```rust
    fn print_table<T>(
        headers: &[&str],
        widths: &[usize],
        items: &[T],
        row_formatter: impl Fn(&T, usize) -> Vec<String>,
    )
    ```
  * Replace the duplicate table formatting blocks in both CLI command handlers with calls to this helper.

---

## 2. Refactoring Tasks

1. Define `print_table` in `youtube-client/src/main.rs`.
2. Update CLI subcommands to utilize the new formatting helper.
