use std::collections::HashMap;
use std::fmt::Debug;

pub trait PrettyPrint {
    fn pretty_print(&self) -> String;
}

fn pprint(sorted_strings: Vec<String>, start_bracket: &str, end_bracket: &str) -> String {
    const MAX_LEN: usize = 125;
    if sorted_strings.is_empty() {
        return format!("{}{}", start_bracket, end_bracket);
    }

    let max = sorted_strings
        .iter()
        .map(|s| s.chars().count())
        .max()
        .unwrap();
    let n = (MAX_LEN / max).max(1);
    format!(
        "{start} {val} {end}",
        start = start_bracket,
        end = end_bracket,
        val = sorted_strings
            .chunks(n)
            .into_iter()
            .enumerate()
            .map(|(i, chunk)| {
                format!(
                    "{}{}",
                    if i == 0 { "" } else { "  " },
                    chunk
                        .into_iter()
                        .map(|s| format!("{:>width$}", s, width = max))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    )
}

impl<T: Ord + Debug, V: Debug> PrettyPrint for HashMap<T, V> {
    fn pretty_print(&self) -> String {
        let mut kvps = self.iter().collect::<Vec<_>>();
        kvps.sort_by_key(|kvp| kvp.0);
        let texts: Vec<String> = kvps
            .into_iter()
            .map(|(key, val)| format!("{:?}: {:?}", key, val))
            .collect();
        pprint(texts, "{", "}")
    }
}

impl<T: Ord + Debug> PrettyPrint for Vec<T> {
    fn pretty_print(&self) -> String {
        let mut s: Vec<_> = self.iter().collect();
        s.sort();
        let texts: Vec<String> = s.into_iter().map(|val| format!("{:?}", val)).collect();
        pprint(texts, "[", "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let map: HashMap<usize, usize> = [(2, 4), (3, 3), (1, 6), (5, 1)].into_iter().collect();
        println!("{}", map.pretty_print());
        assert_eq!(map.pretty_print(), "{ 1: 6, 2: 4, 3: 3, 5: 1 }");

        let map: HashMap<usize, usize> = [
            (2, 4),
            (3, 3),
            (1, 6),
            (5, 1),
            (13, 2),
            (52, 2),
            (123, 2),
            (4321, 5),
            (342432, 2),
            (42314, 1),
            (432, 2),
            (4, 234),
            (4235, 23),
            (752, 1),
            (32423, 2),
            (43214, 2),
            (453212, 23),
            (421, 2),
        ]
        .into_iter()
        .collect();
        println!("{}", map.pretty_print());
        assert_eq!(map.pretty_print(), "{       1: 6,       2: 4,       3: 3,     4: 234,       5: 1,      13: 2,      52: 2,     123: 2,     421: 2,     432: 2,     752: 1,   4235: 23
     4321: 5,   32423: 2,   42314: 1,   43214: 2,  342432: 2, 453212: 23 }");

        let vec: Vec<usize> = vec![
            2, 3, 1, 5, 13, 52, 123, 4321, 342432, 42314, 432, 4, 4235, 752, 32423, 43214, 453212,
            421, 1223, 253, 256, 7563, 24,
        ];
        println!("{}", vec.pretty_print());
        assert_eq!(vec.pretty_print(), "[      1,      2,      3,      4,      5,     13,     24,     52,    123,    253,    256,    421,    432,    752,   1223,   4235,   4321,   7563,  32423,  42314
   43214, 342432, 453212 ]");
    }
}
