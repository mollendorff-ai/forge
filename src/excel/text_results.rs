//! Formula cells whose cached result is text that looks like something else.
//!
//! The workbook writer types a cached result by reading it: `010` becomes the
//! number 10, `TRUE` a boolean, `#N/A` an error. A viewer that does not
//! recalculate (Quick Look, iOS Files) then shows 10 where the formula gives
//! the text `010`. The writer has no way to say "this is text", so the saved
//! workbook is corrected here, cell by cell.

use super::workbook_refs::column_letter;
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

/// Mark the given cells as text in a workbook the writer saved.
///
/// The bytes are the writer's own: a well-formed archive whose worksheets are
/// UTF-8, each formula cell written as `<c r=".."..><f>..</f><v>..</v></c>`.
/// Reading them back and writing them into memory cannot fail.
pub(super) fn mark_as_text(bytes: Vec<u8>, cells: &[TextResult]) -> Vec<u8> {
    const WRITER: &str = "a workbook the writer saved";
    if cells.is_empty() {
        return bytes;
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect(WRITER);
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).expect(WRITER);
        // The writer stores files only, no directory entries
        let name = file.name().expect(WRITER).to_string();
        let mut data = Vec::new();
        file.read_to_end(&mut data).expect(WRITER);
        // The writer names the sheets by position from 1: `xl/worksheets/sheet3.xml` is 2
        let marked: Vec<&TextResult> = cells
            .iter()
            .filter(|c| name == format!("xl/worksheets/sheet{}.xml", c.sheet + 1))
            .collect();
        if !marked.is_empty() {
            let xml = String::from_utf8(data).expect(WRITER);
            data = patch_sheet(&xml, &marked).into_bytes();
        }
        out.start_file(name, options).expect(WRITER);
        out.write_all(&data).expect(WRITER);
    }
    out.finish().expect(WRITER).into_inner()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// One `<c>` element the writer wrote for a formula, made a text result:
/// `t="str"` and the text as its value.
fn text_cell(element: &str, text: &str) -> String {
    const FORMULA: &str = "a formula cell the writer wrote";
    let tag_end = element.find('>').expect(FORMULA);
    let (tag, inner) = (&element[..tag_end], &element[tag_end + 1..]);
    // Drop the type the writer chose
    let tag = tag.find(" t=\"").map_or_else(
        || tag.to_string(),
        |at| {
            let value = at + 4;
            let close = tag[value..].find('"').expect(FORMULA) + value + 1;
            format!("{}{}", &tag[..at], &tag[close..])
        },
    );
    let from = inner.find("<v>").expect(FORMULA) + 3;
    let to = inner.find("</v>").expect(FORMULA);
    format!(
        "{tag} t=\"str\">{}{}{}",
        &inner[..from],
        escape(text),
        &inner[to..]
    )
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
        // The writer wrote every cell given here, as a formula: never `<c .. />`
        let start = xml.find(&start_tag).expect("a cell the writer wrote");
        let end = start + xml[start..].find("</c>").expect("a cell the writer wrote") + 4;
        patches.push((start, end, text_cell(&xml[start..end], &cell.text)));
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
