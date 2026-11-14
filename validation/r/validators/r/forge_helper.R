# forge_helper.R - run a formula through the real Forge binary.
#
# The validators compare Forge's answer with R's. Forge's answer must come
# from Forge: set FORGE_BIN, or have `forge` on the PATH. If Forge cannot be
# run, the script stops, so a missing binary can never look like a pass.

forge_bin <- function() {
  bin <- Sys.getenv("FORGE_BIN", unset = "")
  if (bin == "") bin <- Sys.which("forge")[[1]]
  if (bin == "" || !file.exists(bin)) {
    cat("ERROR: Forge binary not found. Set FORGE_BIN or put forge on the PATH.\n")
    quit(status = 2)
  }
  bin
}

# Evaluate one Excel-style formula with `forge calculate`.
# Returns the text Forge printed after `assumptions.r = `, or NA_character_
# when Forge returns an error or prints no result.
forge_eval_text <- function(formula) {
  tmp <- tempfile(fileext = ".yaml")
  on.exit(unlink(c(tmp, paste0(tmp, ".bak"))), add = TRUE)
  escaped <- gsub("\"", "\\\\\"", formula)
  writeLines(c(
    "_forge_version: \"1.0.0\"",
    "assumptions:",
    "  r:",
    "    value: null",
    sprintf("    formula: \"=%s\"", escaped)
  ), tmp)
  out <- suppressWarnings(system2(forge_bin(), c("calculate", tmp), stdout = TRUE, stderr = TRUE))
  hit <- grep("assumptions\\.r = ", out, value = TRUE)
  if (length(hit) == 0) return(NA_character_)
  trimws(sub(".*assumptions\\.r = ", "", hit[[1]]))
}

# Same, as a number, or NA when Forge returns an error or no number.
forge_eval <- function(formula) {
  suppressWarnings(as.numeric(forge_eval_text(formula)))
}

# Same, for a formula that returns a date. Forge prints a date as text
# (2024-02-29, ADR-060); a date before 1900-03-01 is an Excel serial number.
forge_eval_date <- function(formula) {
  text <- forge_eval_text(formula)
  if (is.na(text)) return(as.Date(NA))
  if (grepl("^[0-9]{4}-[0-9]{2}-[0-9]{2}$", text)) return(as.Date(text))
  serial <- suppressWarnings(as.numeric(text))
  if (is.na(serial)) return(as.Date(NA))
  as.Date(serial, origin = "1899-12-30")
}
