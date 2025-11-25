; Forge YAML Syntax Highlighting for Zed
; Uses tree-sitter-yaml grammar

; Comments
(comment) @comment

; Keys
(block_mapping_pair
  key: (flow_node) @property)

; Strings
(single_quote_scalar) @string
(double_quote_scalar) @string

; Numbers
(integer_scalar) @number
(float_scalar) @number

; Booleans
(boolean_scalar) @constant.builtin

; Null
(null_scalar) @constant.builtin

; Forge formulas (strings starting with =)
((double_quote_scalar) @string.special
  (#match? @string.special "^\"=.*\"$"))

; Excel functions in formulas
((double_quote_scalar) @function.builtin
  (#match? @function.builtin "(SUM|AVERAGE|COUNT|MAX|MIN|IF|AND|OR|SUMIF|COUNTIF|ROUND|INDEX|MATCH)"))
