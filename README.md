# `csv2struct`

Generates struct definitions from CSV using some very basic rules.

# Example

```bash
$ cat test.csv
foo,bar,baz,qux
1,2,3,green
4.4,5,6,red
7.2,,8,blue

$ cat test.csv | csv2struct
#[derive(Debug, Clone, Copy, Eq)]
pub struct Record {
    pub foo: f32,
    pub bar: Option<i32>,
    pub baz: i32,
    pub qux: Qux,
}

#[derive(Debug, Clone, Copy, Eq)]
pub enum Qux {
    Green,
    Red,
    Blue,
}
```

There are two sets of rules at play. First, we apply the following set of
rules to each value in each column and record the results. 

```
if value == ""                        => Empty
if let Some(_) = value.parse::<i32>() => Integer
if let Some(_) = value.parse::<f32>() => Real
else                                  => Factor(value)
```

Next, we apply the following rules to the results for each column:

- If any of the values were parsed as Factor; then treat the column as a factor.
- Otherwise, if not all of the values were parsed as Integer; then treat the column as real.
- Otherwise, treat the column as integer.
- If any of the values were missing, then apply the above rules to the values
  that were present, and wrap the result in `Option`.

Finally, we generate a struct definition with one field for each column. For
factors, we generate an enum as well.

