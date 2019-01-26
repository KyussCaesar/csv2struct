//! # `csv2struct`
//!
//! Generates struct definitions from CSV using some very basic rules.
//!
//! # Example
//!
//! ```bash
//! $ cat test.csv
//! foo,bar,baz,qux
//! 1,2,3,green
//! 4.4,5,6,red
//! 7.2,,8,blue
//!
//! $ cat test.csv | csv2struct
//! #[derive(Debug, Clone, Copy, Eq)]
//! pub struct Record {
//!     pub foo: f32,
//!     pub bar: Option<i32>,
//!     pub baz: i32,
//!     pub qux: Qux,
//! }
//! 
//! #[derive(Debug, Clone, Copy, Eq)]
//! pub enum Qux {
//!     Green,
//!     Red,
//!     Blue,
//! }
//! ```
//!
//! There are two sets of rules at play. First, we apply the following set of
//! rules to each value in each column and record the results. 
//!
//! ```
//! if value == ""                        => Empty
//! if let Some(_) = value.parse::<i32>() => Integer
//! if let Some(_) = value.parse::<f32>() => Real
//! else                                  => Factor(value)
//! ```
//!
//! Next, we apply the following rules to the results for each column:
//!
//! - If any of the values were parsed as Factor; then treat the column as a factor.
//! - Otherwise, if not all of the values were parsed as Integer; then treat the column as real.
//! - Otherwise, treat the column as integer.
//! - If any of the values were missing, then apply the above rules to the values
//!   that were present, and wrap the result in `Option`.
//!
//! Finally, we generate a struct definition with one field for each column. For
//! factors, we generate an enum as well.

use std::io;
use std::collections::HashMap;

use csv;

use inflector::Inflector;

/// Result alias.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error
{
    Msg(String),
}

impl<'a, T: ToString> From<T> for Error
{
    fn from(t: T) -> Self
    {
        Error::Msg(t.to_string())
    }
}

/// Represents an attempt at type inference.
#[derive(Debug)]
struct RecordType
{
    // the fields in the record
    fields: Vec<Field>
}

impl RecordType
{
    fn with(fields: Vec<Field>) -> Self
    {
        Self
        {
            fields
        }
    }

}

/// A field in a record.
#[derive(Debug, Clone)]
struct Field
{
    // the name of the field
    name: String,

    // the type of the field
    kind: FieldKind,
}

impl Field
{
    fn with(name: String, kind: FieldKind) -> Self
    {
        Self
        {
            name,
            kind,
        }
    }
}


/// Represents the different kinds of fields that a record can have
#[derive(Debug, Clone)]
enum FieldKind
{
    Integer,
    Real,
    Factor(String),
    Empty,
}

impl FieldKind
{
    fn parse(f: &str) -> Self
    {
        if f == ""
        {
            FieldKind::Empty
        }

        else
        {
                f.parse::<i32>().map(|_i| FieldKind::Integer)
            .or(f.parse::<f32>().map(|_i| FieldKind::Real))
            .unwrap_or(FieldKind::Factor(f.to_string()))
        }
    }
}

/// Keeps track of, for each field, what types we have seen for it.
/// For example, processing
///
/// ```
/// a,b,c
/// 1,2,red
/// 4.4,5,green
/// ```
///
/// would yield an index
///
/// ```
/// [
///     (a, [Integer,       Real           ]),
///     (b, [Integer,       Integer        ]),
///     (c, [Factor("red"), Factor("green")]),
/// ]
/// ```
///
/// This is then used to generate the type definitions.
#[derive(Debug)]
struct Index
{
    inner: Vec<(String, Vec<FieldKind>)>,
}

impl Index
{
    fn new() -> Self
    {
        Self { inner: Vec::new() }
    }

    fn add(&mut self, rt: RecordType)
    {
        // add self fields to index
        for field in rt.fields.iter()
        {
            match self.inner.iter_mut().find(|i| i.0 == field.name)
            {
                Some((_s, ref mut v)) =>
                    v.push(field.kind.clone()),

                None =>
                    self.inner.push(
                        (field.name.clone(), vec![field.kind.clone()])
                    ),
            }
        }
    }

    fn to_struct_defs(self)
    {
        println!("#[derive(Debug, Clone, Copy, Eq)]");
        print!("pub struct Record {{\n");

        let mut factor_defs = Vec::new();

        for (name, kinds) in self.inner.into_iter()
        {
            print!("    pub {}: ", name);

            // if any are factor -> factor
            // else if not all are integer -> real
            // else -> integer
            // 
            // if any are empty, then it's Option of the above

            let test_empty =
                kinds.iter()
                .any(|k| match k { FieldKind::Empty => true, _ => false });

            let test_factor =
                kinds.iter()
                .filter_map(|k| match k { FieldKind::Empty => None, _ => Some(k) })
                .any(|k| match k { FieldKind::Factor(_) => true, _ => false });

            let test_real =
                kinds.iter()
                .filter_map(|k| match k { FieldKind::Empty => None, _ => Some(k) })
                .any(|k| match k { FieldKind::Integer => false, _ => true });

            let mut type_name = String::new();

            if test_factor
            {
                type_name = name.to_pascal_case();
                let mut factor_def = "#[derive(Debug, Clone, Copy, Eq)]\n".to_string();
                factor_def.extend(format!("pub enum {} {{\n", type_name).chars());

                kinds.into_iter()
                .filter_map(|k| match k { FieldKind::Factor(s) => Some(s), _ => None })
                .for_each(|level|
                    factor_def.extend(
                        format!("    {},\n", level.to_pascal_case()).chars()
                    )
                );

                factor_def.extend("}\n\n".chars());

                factor_defs.push(factor_def);
            }

            else if test_real
            {
                type_name = String::from("f32");
            }

            else
            {
                type_name = String::from("i32");
            }

            if test_empty
            {
                type_name = format!("Option<{}>", type_name);
            }

            print!("{},\n", type_name);
        }

        print!("}}\n\n");

        for factor_def in factor_defs.into_iter()
        {
            print!("{}", factor_def);
        }
    }
}

fn main() -> Result<()> {
    let mut rdr = csv::Reader::from_reader(io::stdin());

    let headers: Vec<String> =
        rdr.headers().unwrap().iter()
        .map(|h| h.to_string())
        .collect();

    // this is responsible for, for each field, keeping track of what type it
    // looks like
    let mut index = Index::new();

    // for each record, see what the RecordType looks like
    for result in rdr.deserialize()
    {
        let record: HashMap<String, String> = result?;

        let mut fields = Vec::new();

        for header in &headers
        {
            fields.push(
                Field::with(
                    header.to_string(),
                    FieldKind::parse(record.get(header).unwrap())
                )
            );
        }

        index.add(RecordType::with(fields));
    }

    index.to_struct_defs();

    Ok(())
}

