//! Tests for the function registry
//! Extracted from registry.rs to keep file under 1500 lines

#[cfg(test)]
mod tests {
    use crate::functions::registry::*;

    #[test]
    fn test_demo_count() {
        // 48 demo functions (16 Math, 5 Aggregation, 5 Logical, 9 Text, 6 Date, 1 Lookup, 6 Trig)
        // INDEX and MATCH removed (require array context, not v1.0.0 compatible)
        assert_eq!(count_demo(), 48, "Demo should have 48 functions");
    }

    #[test]
    fn test_enterprise_count() {
        // 173 total functions (includes aliases like AVG, CONCATENATE, 6 MC.* functions)
        assert_eq!(
            count_enterprise(),
            173,
            "Enterprise should have 173 functions"
        );
    }

    #[test]
    fn test_demo_functions_are_subset() {
        for f in demo_functions() {
            assert!(f.demo, "Demo function {} should have demo=true", f.name);
        }
    }

    #[test]
    fn test_math_demo_count() {
        // 16 Math demo: 9 original + 7 new (EXP, LN, LOG10, INT, SIGN, TRUNC, PI)
        let count = demo_by_category(Category::Math).count();
        assert_eq!(count, 16, "Math should have 16 demo functions");
    }

    #[test]
    fn test_trig_demo_count() {
        // 6 Trig demo: SIN, COS, TAN, ASIN, ACOS, ATAN
        let count = demo_by_category(Category::Trigonometric).count();
        assert_eq!(count, 6, "Trig should have 6 demo functions");
    }

    #[test]
    fn test_find_function() {
        let sum = find_function("SUM");
        assert!(sum.is_some());
        assert!(sum.unwrap().demo);

        let npv = find_function("NPV");
        assert!(npv.is_some());
        assert!(!npv.unwrap().demo);
    }

    #[test]
    fn test_is_demo_function() {
        assert!(is_demo_function("SUM"));
        assert!(is_demo_function("IF"));
        assert!(!is_demo_function("NPV"));
        assert!(!is_demo_function("IRR"));
    }

    #[test]
    fn test_scalar_count() {
        // Scalar functions work without array context (v1.0.0 compatible)
        let scalar = count_scalar();
        let array_only = count_array_only();
        assert_eq!(
            scalar + array_only,
            173,
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
    fn test_demo_functions_are_scalar() {
        // CRITICAL: All demo functions must be scalar (v1.0.0 compatible)
        // Demo is for sales/evaluation - must work without tables/arrays
        let non_scalar_demo: Vec<_> = demo_functions()
            .filter(|f| !f.scalar)
            .map(|f| f.name)
            .collect();
        assert!(
            non_scalar_demo.is_empty(),
            "Demo functions must be scalar (v1.0.0 compatible), but found: {:?}",
            non_scalar_demo
        );
    }

    #[test]
    fn test_array_only_functions() {
        // Verify key array-only functions are correctly marked
        let array_funcs = ["UNIQUE", "FILTER", "SORT", "SUMIF", "VLOOKUP"];
        for name in array_funcs {
            let f = find_function(name).unwrap_or_else(|| panic!("{} should exist", name));
            assert!(!f.scalar, "{} should be scalar=false", name);
        }
    }
}
