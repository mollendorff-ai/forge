//! A small dense linear system, solved by Gaussian elimination with partial
//! pivoting: the indifference equations of a mixed equilibrium, and the
//! expected value of a match that ends at random (ADR-068).

/// A pivot smaller than this, relative to the largest entry of its column,
/// means the system has no single solution.
const SINGULAR: f64 = 1e-12;

/// Solve `a x = b` for a square `a`. `None` when `a` is singular: the
/// equations have no solution or more than one.
#[must_use]
pub fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    let scale = a
        .iter()
        .flatten()
        .fold(0.0_f64, |most, v| most.max(v.abs()));
    if scale == 0.0 {
        return None;
    }
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .unwrap_or(col);
        if a[pivot][col].abs() <= SINGULAR * scale {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let (above, below) = a.split_at_mut(col + 1);
        let pivot_row = &above[col];
        for (offset, target) in below.iter_mut().enumerate() {
            let factor = target[col] / pivot_row[col];
            if factor != 0.0 {
                for (t, p) in target[col..].iter_mut().zip(&pivot_row[col..]) {
                    *t = factor.mul_add(-p, *t);
                }
                let row = col + 1 + offset;
                b[row] = factor.mul_add(-b[col], b[row]);
            }
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let known: f64 = (row + 1..n).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - known) / a[row][row];
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_a_system_that_needs_a_row_swap() {
        // By hand: y = 2, then x + y = 3 gives x = 1, and the zero pivot in
        // the first column needs the rows swapped.
        let x = solve(vec![vec![0.0, 1.0], vec![1.0, 1.0]], vec![2.0, 3.0]).unwrap();
        assert!(
            (x[0] - 1.0).abs() < 1e-12 && (x[1] - 2.0).abs() < 1e-12,
            "{x:?}"
        );
    }

    #[test]
    fn solves_three_equations() {
        // By hand: 2x + y - z = 8, -3x - y + 2z = -11, -2x + y + 2z = -3
        // has x = 2, y = 3, z = -1.
        let x = solve(
            vec![
                vec![2.0, 1.0, -1.0],
                vec![-3.0, -1.0, 2.0],
                vec![-2.0, 1.0, 2.0],
            ],
            vec![8.0, -11.0, -3.0],
        )
        .unwrap();
        for (got, want) in x.iter().zip([2.0, 3.0, -1.0]) {
            assert!((got - want).abs() < 1e-12, "{x:?}");
        }
    }

    #[test]
    fn a_singular_system_has_no_answer() {
        // The second row is twice the first
        assert!(solve(vec![vec![1.0, 2.0], vec![2.0, 4.0]], vec![1.0, 2.0]).is_none());
        assert!(solve(vec![vec![0.0, 0.0], vec![0.0, 0.0]], vec![0.0, 0.0]).is_none());
        assert!(solve(Vec::new(), Vec::new()).is_none());
    }
}
