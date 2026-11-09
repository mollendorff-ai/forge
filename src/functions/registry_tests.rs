//! Tests for the function registry
//! Extracted from registry.rs to keep file under 1500 lines

#[cfg(test)]
mod tests {
    use crate::functions::registry::*;

    #[test]
    fn test_function_count() {
        assert_eq!(count_all(), 174, "Registry should have 174 functions");
    }

    #[test]
    fn test_find_function() {
        let sum = find_function("SUM");
        assert!(sum.is_some());

        let npv = find_function("NPV");
        assert!(npv.is_some());
    }

    #[test]
    fn test_scalar_count() {
        // Scalar functions work without array context (v1.0.0 compatible)
        let scalar = count_scalar();
        let array_only = count_array_only();
        assert_eq!(
            scalar + array_only,
            174,
            "Scalar + array-only should equal total"
        );
        // 24 array-only functions:
        // Array (5): UNIQUE, FILTER, SORT, SEQUENCE, RANDARRAY
        // Conditional (6): SUMIF, SUMIFS, COUNTIF, COUNTIFS, AVERAGEIF, AVERAGEIFS
        // Aggregation (5): MAXIFS, MINIFS, RANK.EQ, LARGE, SMALL
        // Statistical (3): PERCENTILE, QUARTILE, CORREL
        // Lookup (5): INDEX, MATCH, VLOOKUP, HLOOKUP, XLOOKUP, OFFSET, INDIRECT
        assert!(
            array_only >= 20,
            "Should have at least 20 array-only functions"
        );
    }

    #[test]
    fn test_array_only_functions() {
        // Verify key array-only functions are correctly marked
        let array_funcs = ["UNIQUE", "FILTER", "SORT", "SUMIF", "VLOOKUP"];
        for name in array_funcs {
            let f = find_function(name).unwrap_or_else(|| panic!("{name} should exist"));
            assert!(!f.scalar, "{name} should be scalar=false");
        }
    }
}
