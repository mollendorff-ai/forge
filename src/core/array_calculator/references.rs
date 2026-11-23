//! What a name in a formula reads, as the calculator reads it: `rename`
//! finds a name's references with this, so it renames exactly what Forge
//! reads (ADR-075).

use super::names::ScalarName;
use super::ArrayCalculator;
use crate::types::ParsedModel;

/// What a name in a formula reads
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Named {
    /// A scalar, by its full name
    Scalar(String),
    /// A table's column
    Column { table: String, column: String },
    /// `@alias.name`: a name in the file the model includes as `alias`, as
    /// that file's model names it
    Included { alias: String, name: String },
    /// A game's result, `<game>.<rest>` (`price_war.dominated.lune.hold`):
    /// the game, and the rest as the game names its results
    Game { game: String, rest: String },
    /// Anything else: a game's result, a name the model does not have
    Other,
}

/// Where a formula is written, which decides what its short names read
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place<'a> {
    /// The formula of the scalar of this full name: a short name reads the
    /// scalar's group first
    Scalar(&'a str),
    /// A column's formula in this table: a bare name reads the table's own
    /// columns first
    Column(&'a str),
    /// Anywhere else, a game's payoff for one: as at the top level
    Elsewhere,
}

/// A model's names, read as the calculator reads them
pub struct Reader {
    calculator: ArrayCalculator,
}

impl Reader {
    #[must_use]
    pub fn new(model: ParsedModel) -> Self {
        Self {
            calculator: ArrayCalculator::new(model),
        }
    }

    /// What `name`, written in a formula at `place`, reads: a game's result
    /// when its first part is a game, then a column of its own table, then a
    /// scalar as the calculator resolves it (its group first, then the top
    /// level, then the only scalar of that last name), then a table's column.
    #[must_use]
    pub fn read(&self, name: &str, place: Place<'_>) -> Named {
        if let Some(rest) = name.strip_prefix('@') {
            return rest
                .split_once('.')
                .map_or(Named::Other, |(alias, name)| Named::Included {
                    alias: alias.to_string(),
                    name: name.to_string(),
                });
        }
        let model = &self.calculator.model;
        if let Some((game, rest)) = name
            .split_once('.')
            .filter(|(g, _)| model.games.contains_key(*g))
        {
            return Named::Game {
                game: game.to_string(),
                rest: rest.to_string(),
            };
        }
        let column = |table: &str, column: &str| {
            model.tables.get(table).is_some_and(|t| {
                t.columns.contains_key(column) || t.row_formulas.contains_key(column)
            })
        };
        let named_column = |table: &str, name: &str| Named::Column {
            table: table.to_string(),
            column: name.to_string(),
        };
        if let Place::Column(table) = place {
            if !name.contains('.') && column(table, name) {
                return named_column(table, name);
            }
        }
        let section = match place {
            Place::Scalar(scalar) => scalar.rsplit_once('.').map(|(group, _)| group),
            _ => None,
        };
        if let ScalarName::Found(full) = self.calculator.resolve_scalar_name(name, section) {
            return Named::Scalar(full);
        }
        name.split_once('.')
            .filter(|(table, name)| column(table, name))
            .map_or(Named::Other, |(table, name)| named_column(table, name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Column, ColumnValue, Table, Variable};

    fn model() -> ParsedModel {
        let mut model = ParsedModel::new();
        for name in [
            "rate",
            "year.price",
            "year.units",
            "other.price",
            "costs.rent",
        ] {
            model.add_scalar(
                name.to_string(),
                Variable::new(name.to_string(), Some(1.0), None),
            );
        }
        let mut months = Table::new("months".to_string());
        months.add_column(Column::new(
            "month".to_string(),
            ColumnValue::Number(vec![1.0]),
        ));
        months.add_row_formula("rent".to_string(), "=month".to_string());
        model.add_table(months);
        model
    }

    fn scalar(name: &str) -> Named {
        Named::Scalar(name.to_string())
    }

    fn column(table: &str, column: &str) -> Named {
        Named::Column {
            table: table.to_string(),
            column: column.to_string(),
        }
    }

    #[test]
    fn a_name_reads_what_the_calculator_reads_where_it_is_written() {
        let reader = Reader::new(model());
        let read = |name, place| reader.read(name, place);
        // Its own group first, then the top level, then the only scalar of
        // that last name
        assert_eq!(
            read("price", Place::Scalar("year.units")),
            scalar("year.price")
        );
        assert_eq!(
            read("price", Place::Scalar("other.x")),
            scalar("other.price")
        );
        assert_eq!(read("rate", Place::Scalar("year.units")), scalar("rate"));
        assert_eq!(read("units", Place::Elsewhere), scalar("year.units"));
        // Two scalars of that name, and no group to choose: neither
        assert_eq!(read("price", Place::Elsewhere), Named::Other);
        assert_eq!(read("year.price", Place::Elsewhere), scalar("year.price"));
        // A column's own table first: `rent` is its column, not costs.rent
        assert_eq!(
            read("rent", Place::Column("months")),
            column("months", "rent")
        );
        assert_eq!(read("rent", Place::Elsewhere), scalar("costs.rent"));
        assert_eq!(
            read("month", Place::Column("months")),
            column("months", "month")
        );
        assert_eq!(
            read("months.month", Place::Elsewhere),
            column("months", "month")
        );
        assert_eq!(read("months.nothing", Place::Elsewhere), Named::Other);
        assert_eq!(read("nothing", Place::Column("months")), Named::Other);
        assert_eq!(read("SUM", Place::Elsewhere), Named::Other);
        assert_eq!(
            read("@costs.year.price", Place::Elsewhere),
            Named::Included {
                alias: "costs".to_string(),
                name: "year.price".to_string()
            }
        );
        assert_eq!(read("@costs", Place::Elsewhere), Named::Other);
    }

    #[test]
    fn a_name_under_a_game_reads_its_result() {
        let text = "_forge_version: \"5.5.0\"\nr: 2\ngames:\n  pd:\n    players: [a, b]\n    strategies: [c, d]\n    symmetric: true\n    payoffs:\n      c: {c: =r, d: 0}\n      d: {c: 3, d: 1}\n";
        let model =
            crate::parser::parse_model_text("m.yaml", text, &std::collections::HashMap::new())
                .unwrap();
        let reader = Reader::new(model);
        assert_eq!(
            reader.read("pd.dominated.a.c", Place::Elsewhere),
            Named::Game {
                game: "pd".to_string(),
                rest: "dominated.a.c".to_string()
            }
        );
        // The game alone is no result, and nor is a name under another word
        assert_eq!(reader.read("pd", Place::Elsewhere), Named::Other);
        assert_eq!(reader.read("pq.equilibria", Place::Elsewhere), Named::Other);
        assert_eq!(reader.read("r", Place::Elsewhere), scalar("r"));
    }
}
