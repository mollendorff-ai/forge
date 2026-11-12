//! Formula cells whose cached result is text that looks like something else.
//!
//! The workbook writer types a cached result by reading it: `010` becomes the
//! number 10, `TRUE` a boolean, `#N/A` an error. A viewer that does not
//! recalculate (Quick Look, iOS Files) then shows 10 where the formula gives
//! the text `010`. The writer has no way to say "this is text", so the saved
//! workbook is corrected here, cell by cell.

use super::workbook_refs::column_letter;
use crate::error::{ForgeError, ForgeResult};
use std::io::{Cursor, Read, Write};

/// Where a formula cell is: the sheet by position, then the row and column.
pub(super) struct TextCell {
    pub sheet: usize,
    pub position: (u32, u16),
}

/// A formula cell whose cached result is text the writer would type as
/// something else.
pub(super) struct TextResult {
    /// Position of the sheet in the workbook, from 0
    pub sheet: usize,
    /// Row from 0 and column from 0
    pub position: (u32, u16),
    /// The text the formula gives
    pub text: String,
}

/// Whether the writer would read this text as a number, a boolean or an error.
pub(super) fn looks_like_another_type(text: &str) -> bool {
    text.parse::<f64>().is_ok() || text == "TRUE" || text == "FALSE" || text.starts_with('#')
}

fn zip_error(e: impl std::fmt::Display) -> ForgeError {
    ForgeError::IO(format!("Failed to mark text results in the workbook: {e}"))
}

/// Mark the given cells as text in a saved workbook.
///
/// # Errors
///
/// Returns an error if the workbook cannot be read back or written again.
pub(super) fn mark_as_text(bytes: Vec<u8>, cells: &[TextResult]) -> ForgeResult<Vec<u8>> {
    if cells.is_empty() {
        return Ok(bytes);
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(zip_error)?;
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(zip_error)?;
        let name = file.name().map_err(zip_error)?.to_string();
        if file.is_dir() {
            out.add_directory(name, options).map_err(zip_error)?;
            continue;
        }
        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(zip_error)?;
        if let Some(sheet) = sheet_position(&name) {
            let marked: Vec<&TextResult> = cells.iter().filter(|c| c.sheet == sheet).collect();
            if !marked.is_empty() {
                let xml = String::from_utf8(data).map_err(zip_error)?;
                data = patch_sheet(&xml, &marked).into_bytes();
            }
        }
        out.start_file(name, options).map_err(zip_error)?;
        out.write_all(&data).map_err(zip_error)?;
    }
    Ok(out.finish().map_err(zip_error)?.into_inner())
}

/// The position of a sheet from its file name (`xl/worksheets/sheet3.xml` is 2).
fn sheet_position(name: &str) -> Option<usize> {
    let number = name
        .strip_prefix("xl/worksheets/sheet")?
        .strip_suffix(".xml")?
        .parse::<usize>()
        .ok()?;
    number.checked_sub(1)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// One `<c>` element made a text result: `t="str"` and the text as its value.
fn text_cell(element: &str, text: &str) -> Option<String> {
    let tag_end = element.find('>')?;
    let (tag, inner) = (&element[..tag_end], &element[tag_end + 1..]);
    // Drop the type the writer chose
    let tag = match tag.find(" t=\"") {
        Some(at) => {
            let value = at + 4;
            let close = tag[value..].find('"')? + value + 1;
            format!("{}{}", &tag[..at], &tag[close..])
        },
        None => tag.to_string(),
    };
    let from = inner.find("<v>")? + 3;
    let to = inner.find("</v>")?;
    Some(format!(
        "{tag} t=\"str\">{}{}{}",
        &inner[..from],
        escape(text),
        &inner[to..]
    ))
}

fn patch_sheet(xml: &str, cells: &[&TextResult]) -> String {
    // (start, end, replacement) of each element, applied from the first
    let mut patches: Vec<(usize, usize, String)> = Vec::new();
    for cell in cells {
        let reference = format!(
            "{}{}",
            column_letter(cell.position.1 as usize),
            cell.position.0 + 1
        );
        let start_tag = format!("<c r=\"{reference}\"");
        let Some(start) = xml.find(&start_tag) else {
            continue;
        };
        let Some(length) = xml[start..].find("</c>") else {
            continue;
        };
        let end = start + length + 4;
        if let Some(replacement) = text_cell(&xml[start..end], &cell.text) {
            patches.push((start, end, replacement));
        }
    }
    patches.sort_by_key(|p| p.0);
    let mut out = String::with_capacity(xml.len());
    let mut at = 0;
    for (start, end, replacement) in patches {
        out.push_str(&xml[at..start]);
        out.push_str(&replacement);
        at = end;
    }
    out.push_str(&xml[at..]);
    out
}
