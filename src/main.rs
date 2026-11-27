use std::io::{self, BufRead, Read, Write};

use clap::Parser;

/*
Print selected parts of lines

cut writes to standard output selected
parts of each line of each input file,
or standard input if no files are given
or for a file name of ‘-’. Synopsis:

```rust
cut option... [file]...
```
In the table which follows, the byte-list, character-list, and field-list are one or more numbers or ranges (two numbers separated by a ..) separated by commas. Bytes, characters, and fields are numbered starting at 1. Incomplete ranges may be given: ..m means ‘1..m’; ‘n..’ means ‘n’ through end of line or last field. The list elements can be repeated, can overlap, and can be specified in any order; but the selected input is written in the same order that it is read, and is written exactly once.


An exit status of zero indicates success, and a nonzero value indicates failure.
*/
#[derive(Parser, Debug, Default)]
#[command(version, about, long_about = None)]
struct Args {
    /// Select for printing only the bytes in positions listed in byte-list. Tabs and backspaces are treated like any other character; they take up 1 byte. If an output delimiter is specified, (see the description of --output-delimiter), then output that string between ranges of selected bytes.
    #[arg(short, long)]
    bytes: Option<String>,
    /// Select for printing only the characters in positions listed in character-list.
    ///
    /// Tabs and backspaces are treated like any other character; they take up 1 character.
    ///
    /// Combining characters are considered as separate characters.
    ///
    /// If an output delimiter is specified, (see the description of --output-delimiter), then output that string between ranges of selected characters.
    #[arg(short, long)]
    characters: Option<String>,
    /// This option is a GNU extension.
    ///
    /// Select for printing the complement of the bytes, characters or fields selected with the -b, -c or -f options.
    ///
    /// In other words, do not print the bytes, characters or fields specified via those options.
    ///
    /// This option is useful when you have many fields and want to print all but a few of them.
    #[arg(long)]
    complement: Option<String>,
    ///With -f, use character as the input field separator (default is TAB).
    ///
    ///If an empty delimiter character is specified then the ASCII NUL character is used to delimit the fields.
    ///
    ///Note the delimiter character can be the same as the line terminator character, which can be used to select ranges of lines. In this case the last line terminator in the input is not treated as a field separator, which is significant with the -s option.
    #[arg(short, long)]
    delimiter: Option<char>,
    ///Select for printing only the fields listed in field-list.
    ///Fields are separated by a TAB character by default. Also print any line that contains no delimiter character, unless the --only-delimited (-s) option is specified.
    ///
    /// The awk command supports more sophisticated field processing, like reordering fields, and handling fields aligned with blank characters.
    ///By default awk uses (and discards) runs of blank characters to separate fields, and ignores leading and trailing blanks.
    ///
    ///awk '{print $2}'      # print the second field
    ///
    ///awk '{print $(NF-1)}' # print the penultimate field
    ///
    ///awk '{print $2,$1}'   # reorder the first two fields
    ///While cut accepts field specifications in arbitrary order, output is always in the order encountered in the file.
    ///
    ///In the unlikely event that awk is unavailable, one can use the join command, to process blank characters as awk does above.
    ///
    ///join -a1 -o 1.2     - /dev/null # print the second field
    ///join -a1 -o 1.2,1.1 - /dev/null # reorder the first two fields
    #[arg(short, long)]
    fields: Option<String>,
    /// Like the -f,--fields option, but also implies -w and --output-delimitor=' '.
    ///
    /// I.e., fields are separated by runs of blank characters, and output fields are separated by a single ASCII space.
    #[arg(short = 'F')]
    spaced_fields: Option<String>,

    /// With --bytes, do not split multi-byte characters. A byte range must encompass the end of a multi-byte character for it to be selected.
    #[arg(short, long)]
    no_partial: Option<String>,

    /// With -f, output fields are separated by string.
    ///
    /// The default with -f is to use the input delimiter, or with -w the TAB character.
    ///
    /// When using -b or -c to select ranges of byte or character offsets (as opposed to ranges of fields), output output_delim_string between non-overlapping ranges of selected bytes.
    #[arg(short = 'O', long)]
    output_delimiter: Option<String>,
    /// For -f, do not print lines that do not contain field separators.
    ///
    /// Normally, any line without a field separator is printed verbatim.
    ///
    /// With --whitespace-delimited=trimmed, lines that contain only leading or trailing blanks will be suppressed.
    #[arg(short = 's', long)]
    only_delimited: bool,

    /// With -f, separate fields with a run of
    /// blank characters (usually space or TAB).
    ///
    /// With ‘trimmed’ do not consider leading
    /// and trailing blanks as field
    /// separators.
    ///
    /// -w is implied with the -F
    /// option.
    ///
    /// Note efficiency of this mode can
    /// be improved if the input fields are
    /// separated by runs of only ASCII blank
    /// characters (space or TAB). Even if
    /// fields contain arbitrary other
    /// characters, you can efficiently search
    /// for ASCII blanks by invoking cut like
    /// ‘LC_ALL=C cut’.
    #[arg(short, long)]
    whitespace_delimited: bool,

    files: Vec<String>,
}

enum RustCutError {
    Io(std::io::Error),
    CutError(CutError),
}

#[derive(PartialEq, Eq)]
enum CutError {
    BufLines(String),
    ArgRange(ArgRangeError),
    MultipleLists,
}

impl std::fmt::Debug for CutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            CutError::BufLines(error) => {
                format!("Error getting the lines from the buf reader: {}", error)
            }
            CutError::ArgRange(err) => match err {
                ArgRangeError::InvalidRangeInput => String::from("Invalid range input provided"),
            },
            CutError::MultipleLists => String::from("only 1 type of list may be specified"),
        };

        f.write_str(&message)
    }
}

impl std::fmt::Debug for RustCutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            RustCutError::Io(error) => format!("Io error: {error}"),
            RustCutError::CutError(CutError::BufLines(error)) => {
                format!("Error getting the lines from the buf reader: {}", error)
            }
            RustCutError::CutError(CutError::ArgRange(err)) => match err {
                ArgRangeError::InvalidRangeInput => String::from("Invalid range input provided"),
            },
            RustCutError::CutError(CutError::MultipleLists) => {
                String::from("only 1 type of list may be specified")
            }
        };
        f.write_str(&message)
    }
}

fn main() -> Result<(), RustCutError> {
    let mut args = Args::parse();

    // add stdin if the files are empty
    if args.files.is_empty() {
        args.files.push("-".to_string());
    }
    for file in &args.files {
        let result = if file == "-" {
            cut(io::stdin(), &mut io::stdout(), &args)
        } else {
            cut(
                std::fs::File::open(file).map_err(RustCutError::Io)?,
                &mut io::stdout(),
                &args,
            )
        };
        result.map_err(RustCutError::CutError)?;
    }
    Ok(())
}

fn cut(r: impl Read, w: &mut impl Write, args: &Args) -> Result<(), CutError> {
    // we don't allow 2 lists
    let arg_ranges = match (
        args.bytes.as_ref(),
        args.characters.as_ref(),
        args.fields.as_ref(),
    ) {
        (None, None, None) => vec![],
        (None, None, Some(s)) | (None, Some(s), None) | (Some(s), None, None) => {
            get_arg_ranges(s).map_err(CutError::ArgRange)?
        }
        _ => return Err(CutError::MultipleLists),
    };

    let arg_ranges = merge_arg_ranges(&arg_ranges);

    read_lines(r, w, &arg_ranges)
}

// we need to merge these arg ranges
// so we have a left and an optional right
// 1..?
// This can be 1..5, 8..9  - so we need 2 separate ranges because they don't overlap
// Or it can be 1..5, 3..7 - so we need a single range as these do overlap
// Or it can be 1..8, 3..5 - so we need a single range as 1..8 overlaps 3..5
//
// So we need to know what ranges we've got already
// 1..5 - we need to store this range
// 8..9 - we then need to store this range
//
// 1..5 - we need to store this range
// 3..7 - then we need to see that the start idx 3 is contained in the range 1..5.
// And then we need to see that end idx 7 is greater than the 5 and so update this overlapping range to take the higher bound
// NOTE: the inverse also applies here, we need to check lower bounds
//
// 1..8 - we need to store this range
// 3..5 - then we need to see that the start idx 3 is contained in the range 1..8 AND the 5
//   is contained in the range 1..8 - we would remove this range as it's already covered
fn merge_arg_ranges(arg_ranges: &[ArgRange]) -> Vec<ArgRange> {
    let mut found_ranges: Vec<ArgRange> = Vec::new();
    'outer: for range in arg_ranges {
        for found_range in &mut found_ranges {
            if is_intersecting(range, found_range) {
                found_range.start = std::cmp::min(range.start, found_range.start);
                found_range.end = max_optionals(range.end, found_range.end);
                continue 'outer;
            }
        }
        found_ranges.push(*range);
    }
    found_ranges
}

fn is_intersecting(a: &ArgRange, b: &ArgRange) -> bool {
    // test for the non intersecting case
    // b: 5..8 a: 1..3
    // a: 5..8 b: 1..3
    !((a.start > b.start && a.end > b.end && test_left_gt_right(a.start, b.end))
        || (b.start > a.start && b.end > a.end && test_left_gt_right(b.start, a.end)))
}

fn test_left_gt_right(left: usize, is_greater_than: Option<usize>) -> bool {
    if let Some(right) = is_greater_than {
        left > right
    } else {
        // right is greater than left
        false
    }
}

fn max_optionals(a: Option<usize>, b: Option<usize>) -> Option<usize> {
    if let (Some(a), Some(b)) = (a, b) {
        Some(std::cmp::max(a, b))
    } else {
        None
    }
}

fn read_lines(r: impl Read, w: &mut impl Write, arg_range: &[ArgRange]) -> Result<(), CutError> {
    let lines_iter = io::BufReader::new(r).lines();
    for line in lines_iter {
        let line = line.map_err(|err| CutError::BufLines(err.to_string()))?;
        let mut range_values = get_range_values(&line, arg_range);
        range_values.push('\n');
        w.write(&range_values.into_bytes())
            .map_err(|err| CutError::BufLines(err.to_string()))?;
    }
    Ok(())
}

fn get_range_values(line: &str, ranges: &[ArgRange]) -> String {
    ranges
        .iter()
        .map(|r| {
            let line_len = line.len();
            let start = r.start - 1;
            let end = r.end.unwrap_or(line_len);
            if line_len < start || end > line_len {
                String::new()
            } else {
                String::from(&line[start..end])
            }
        })
        .collect::<String>()
}

fn get_arg_ranges(s: &str) -> Result<Vec<ArgRange>, ArgRangeError> {
    let mut res = s
        .split(',')
        .map(ArgRange::from_str)
        .collect::<Result<Vec<ArgRange>, ArgRangeError>>()?;

    res.sort_by_key(|range| range.start);
    Ok(res)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ArgRange {
    start: usize,
    end: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ArgRangeError {
    InvalidRangeInput,
}

impl ArgRange {
    fn new(start: usize, end: Option<usize>) -> Self {
        if let Some(end_unwrapped) = end
            && start > end_unwrapped
        {
            Self {
                start: end_unwrapped,
                end: Some(start),
            }
        } else {
            Self { start, end }
        }
    }

    fn from_str(range: &str) -> Result<ArgRange, ArgRangeError> {
        // possible ranges
        // ..m  = 1..m
        // n..  = n..end of line/last field
        // 1..5 = pos 1 to pos 5
        // 2..7 = pos 2 to pos 7
        let ranges = range.split("..").map(|s| s.trim()).collect::<Vec<&str>>();

        match ranges[..] {
            [_, "0"] | ["0", _] => Err(ArgRangeError::InvalidRangeInput),
            [n, ""] => {
                let n = n
                    .parse::<usize>()
                    .map_err(|_| ArgRangeError::InvalidRangeInput)?;
                Ok(ArgRange::new(n, None))
            }
            ["", m] => {
                let m = m
                    .parse::<usize>()
                    .map_err(|_| ArgRangeError::InvalidRangeInput)?;
                Ok(ArgRange::new(1, Some(m)))
            }
            [n, m] => {
                let n = n
                    .parse::<usize>()
                    .map_err(|_| ArgRangeError::InvalidRangeInput)?;
                let m = m
                    .parse::<usize>()
                    .map_err(|_| ArgRangeError::InvalidRangeInput)?;
                Ok(ArgRange::new(n, Some(m)))
            }
            _ => Err(ArgRangeError::InvalidRangeInput),
        }
    }
}

#[cfg(test)]
mod cut_tests {

    use crate::{Args, CutError, cut};

    #[test]
    fn fails_if_multiple_lists() {
        let test_r = String::from("testing");
        let mut w: Vec<u8> = Vec::new();
        let args = Args {
            bytes: Some(String::from("1..2")),
            characters: Some(String::from("1..2")),
            ..Default::default()
        };
        let actual = cut(test_r.as_bytes(), &mut w, &args);
        assert_eq!(Err(CutError::MultipleLists), actual);
    }

    #[test]
    fn simple_range() {
        test_range("testing", "1..2", Ok(()), Some(b"te\n"));
    }

    #[test]
    fn reverse_range() {
        test_range("testing", "2..1", Ok(()), Some(b"te\n"));
    }

    #[test]
    fn multiple_ranges() {
        test_range("testing", "1..2,4..5", Ok(()), Some(b"teti\n"));
    }

    #[test]
    fn multiple_ranges_spaced_comma() {
        test_range("testing", "1..2, 4..5", Ok(()), Some(b"teti\n"));
    }

    #[test]
    fn out_of_order_ranges() {
        test_range("testing", "4..5,1..2", Ok(()), Some(b"teti\n"));
    }

    #[test]
    fn out_of_order_reverse_ranges() {
        test_range("testing", "5..4,2..1", Ok(()), Some(b"teti\n"));
    }

    #[test]
    fn out_of_bounds() {
        test_range("testing", "55..56", Ok(()), Some(b"\n"));
    }

    #[test]
    fn reverse_out_of_bounds() {
        test_range("testing", "56..55", Ok(()), Some(b"\n"));
    }

    fn test_range(
        input: &str,
        range: &str,
        expected_res: Result<(), CutError>,
        expected_bytes: Option<&[u8]>,
    ) {
        let test_r = String::from(input);
        let mut w: Vec<u8> = Vec::new();
        let args = Args {
            characters: Some(String::from(range)),
            ..Default::default()
        };
        let actual = cut(test_r.as_bytes(), &mut w, &args);
        assert_eq!(expected_res, actual);
        if let Some(b) = expected_bytes {
            assert_eq!(b, w);
        }

        let args = Args {
            bytes: Some(String::from(range)),
            ..Default::default()
        };
        let actual = cut(test_r.as_bytes(), &mut w, &args);
        assert_eq!(expected_res, actual);
    }
}

#[cfg(test)]
mod arg_range_tests {
    use crate::{ArgRange, ArgRangeError};

    #[test]
    fn from_str_bounded_range() {
        let range = "1..5";
        let expected = ArgRange::new(1, Some(5));
        let actual = ArgRange::from_str(range);
        assert_eq!(Ok(expected), actual);
    }

    #[test]
    fn from_str_bounded_range_with_non_start_range() {
        let range = "3..5";
        let expected = ArgRange::new(3, Some(5));
        let actual = ArgRange::from_str(range);
        assert_eq!(Ok(expected), actual);
    }

    #[test]
    fn from_str_right_bounded_range() {
        let range = "..5";
        let expected = ArgRange::new(1, Some(5));
        let actual = ArgRange::from_str(range);
        assert_eq!(Ok(expected), actual);
    }

    #[test]
    fn from_str_left_bounded_range() {
        let range = "1..";
        let expected = ArgRange::new(1, None);
        let actual = ArgRange::from_str(range);
        assert_eq!(Ok(expected), actual);
    }

    #[test]
    fn from_str_left_bounded_range_with_non_start_range() {
        let range = "3..";
        let expected = ArgRange::new(3, None);
        let actual = ArgRange::from_str(range);
        assert_eq!(Ok(expected), actual);
    }

    #[test]
    fn out_of_order_ranges_reordered() {
        let range = "5..3";
        let expected = ArgRange::new(3, Some(5));
        let actual = ArgRange::from_str(range);
        assert_eq!(Ok(expected), actual);
    }

    #[test]
    fn invalid_input_gives_error() {
        let range = "0..1";
        let actual = ArgRange::from_str(range);
        assert_eq!(Err(ArgRangeError::InvalidRangeInput), actual);

        let range = "1..0";
        let actual = ArgRange::from_str(range);
        assert_eq!(Err(ArgRangeError::InvalidRangeInput), actual);

        let range = "..";
        let actual = ArgRange::from_str(range);
        assert_eq!(Err(ArgRangeError::InvalidRangeInput), actual);

        let range = "-1..";
        let actual = ArgRange::from_str(range);
        assert_eq!(Err(ArgRangeError::InvalidRangeInput), actual);

        let range = "..-1";
        let actual = ArgRange::from_str(range);
        assert_eq!(Err(ArgRangeError::InvalidRangeInput), actual);

        let range = "123";
        let actual = ArgRange::from_str(range);
        assert_eq!(Err(ArgRangeError::InvalidRangeInput), actual);

        let range = "abc";
        let actual = ArgRange::from_str(range);
        assert_eq!(Err(ArgRangeError::InvalidRangeInput), actual);

        let range = "34uy8902uofdlsajfkdals38..3oi4uoaejflkasjfds";
        let actual = ArgRange::from_str(range);
        assert_eq!(Err(ArgRangeError::InvalidRangeInput), actual);
    }
}

#[cfg(test)]
mod merge_arg_ranges_tests {
    use crate::{ArgRange, merge_arg_ranges};

    #[test]
    fn merges_two_overlapping_ranges() {
        let left_range = ArgRange::new(1, Some(4));
        let right_range = ArgRange::new(3, Some(5));
        let ranges = vec![left_range, right_range];
        let actual_ranges = merge_arg_ranges(&ranges);

        assert_eq!(vec![ArgRange::new(1, Some(5))], actual_ranges);

        let left_range = ArgRange::new(1, Some(4));
        let right_range = ArgRange::new(3, Some(5));
        let ranges_out_of_order = vec![right_range, left_range];
        let actual_ranges = merge_arg_ranges(&ranges_out_of_order);

        assert_eq!(vec![ArgRange::new(1, Some(5))], actual_ranges);
    }
    #[test]
    fn merges_containing_ranges() {
        let outer_range = ArgRange::new(1, Some(5));
        let inner_range = ArgRange::new(3, Some(4));
        let ranges = vec![outer_range, inner_range];
        let actual_ranges = merge_arg_ranges(&ranges);

        assert_eq!(vec![ArgRange::new(1, Some(5))], actual_ranges);

        let outer_range = ArgRange::new(1, Some(5));
        let inner_range = ArgRange::new(3, Some(4));
        let ranges = vec![inner_range, outer_range];
        let actual_ranges = merge_arg_ranges(&ranges);

        assert_eq!(vec![ArgRange::new(1, Some(5))], actual_ranges);
    }

    #[test]
    fn keeps_separate_ranges() {
        let left_range = ArgRange::new(1, Some(5));
        let right_range = ArgRange::new(6, Some(8));
        let ranges = vec![left_range, right_range];
        let actual_ranges = merge_arg_ranges(&ranges);

        assert_eq!(vec![left_range, right_range], actual_ranges);

        let left_range = ArgRange::new(1, Some(5));
        let right_range = ArgRange::new(6, Some(8));
        let ranges = vec![right_range, left_range];
        let actual_ranges = merge_arg_ranges(&ranges);

        assert_eq!(vec![right_range, left_range], actual_ranges);
    }
}
