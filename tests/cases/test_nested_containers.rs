use derive_into::Convert;
use std::collections::HashMap;

// Custom wrapper types
#[derive(Debug, PartialEq, Default, Clone)]
struct Tag(String);

impl From<String> for Tag {
    fn from(s: String) -> Self {
        Tag(s)
    }
}

impl From<Tag> for String {
    fn from(t: Tag) -> Self {
        t.0
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
struct Score(u32);

impl From<u32> for Score {
    fn from(n: u32) -> Self {
        Score(n)
    }
}

impl From<Score> for u32 {
    fn from(s: Score) -> Self {
        s.0
    }
}

// --- Option<Vec<T>> ---

#[derive(Convert, Debug, PartialEq)]
#[convert(into(path = "TargetA"))]
#[convert(try_from(path = "TargetA"))]
struct SourceA {
    tags: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Default)]
struct TargetA {
    tags: Option<Vec<Tag>>,
}

// --- Vec<Option<T>> ---

#[derive(Convert, Debug, PartialEq)]
#[convert(into(path = "TargetB"))]
#[convert(try_from(path = "TargetB"))]
struct SourceB {
    scores: Vec<Option<u32>>,
}

#[derive(Debug, PartialEq, Default)]
struct TargetB {
    scores: Vec<Option<Score>>,
}

// --- Vec<Vec<T>> ---

#[derive(Convert, Debug, PartialEq)]
#[convert(into(path = "TargetC"))]
#[convert(try_from(path = "TargetC"))]
struct SourceC {
    matrix: Vec<Vec<u32>>,
}

#[derive(Debug, PartialEq, Default)]
struct TargetC {
    matrix: Vec<Vec<Score>>,
}

// --- Option<HashMap<K, V>> ---

#[derive(Convert, Debug, PartialEq)]
#[convert(into(path = "TargetD"))]
#[convert(try_from(path = "TargetD"))]
struct SourceD {
    metadata: Option<HashMap<String, u32>>,
}

#[derive(Debug, PartialEq, Default)]
struct TargetD {
    metadata: Option<HashMap<String, Score>>,
}

// --- Option<Option<T>> ---

#[derive(Convert, Debug, PartialEq)]
#[convert(into(path = "TargetE"))]
#[convert(try_from(path = "TargetE"))]
struct SourceE {
    nested_opt: Option<Option<u32>>,
}

#[derive(Debug, PartialEq, Default)]
struct TargetE {
    nested_opt: Option<Option<Score>>,
}

// --- HashMap<K, Vec<V>> ---

#[derive(Convert, Debug, PartialEq)]
#[convert(into(path = "TargetF"))]
#[convert(try_from(path = "TargetF"))]
struct SourceF {
    grouped: HashMap<String, Vec<u32>>,
}

#[derive(Debug, PartialEq, Default)]
struct TargetF {
    grouped: HashMap<String, Vec<Score>>,
}

// --- Unwrap on Option<Vec<T>> ---

#[derive(Convert, Debug, PartialEq)]
#[convert(into(path = "TargetG"))]
struct SourceG {
    #[convert(unwrap)]
    items: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Default)]
struct TargetG {
    items: Vec<Tag>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Option<Vec<T>> tests ---

    #[test]
    fn test_option_vec_into_some() {
        let source = SourceA {
            tags: Some(vec!["rust".to_string(), "macro".to_string()]),
        };
        let target: TargetA = source.into();
        assert_eq!(
            target.tags,
            Some(vec![Tag("rust".to_string()), Tag("macro".to_string())])
        );
    }

    #[test]
    fn test_option_vec_into_none() {
        let source = SourceA { tags: None };
        let target: TargetA = source.into();
        assert_eq!(target.tags, None);
    }

    #[test]
    fn test_option_vec_try_from() {
        let target = TargetA {
            tags: Some(vec![Tag("hello".to_string())]),
        };
        let source = SourceA::try_from(target).unwrap();
        assert_eq!(source.tags, Some(vec!["hello".to_string()]));
    }

    // --- Vec<Option<T>> tests ---

    #[test]
    fn test_vec_option_into() {
        let source = SourceB {
            scores: vec![Some(10), None, Some(20)],
        };
        let target: TargetB = source.into();
        assert_eq!(
            target.scores,
            vec![Some(Score(10)), None, Some(Score(20))]
        );
    }

    #[test]
    fn test_vec_option_try_from() {
        let target = TargetB {
            scores: vec![Some(Score(5)), None],
        };
        let source = SourceB::try_from(target).unwrap();
        assert_eq!(source.scores, vec![Some(5), None]);
    }

    // --- Vec<Vec<T>> tests ---

    #[test]
    fn test_vec_vec_into() {
        let source = SourceC {
            matrix: vec![vec![1, 2], vec![3, 4, 5]],
        };
        let target: TargetC = source.into();
        assert_eq!(
            target.matrix,
            vec![
                vec![Score(1), Score(2)],
                vec![Score(3), Score(4), Score(5)]
            ]
        );
    }

    #[test]
    fn test_vec_vec_try_from() {
        let target = TargetC {
            matrix: vec![vec![Score(10)]],
        };
        let source = SourceC::try_from(target).unwrap();
        assert_eq!(source.matrix, vec![vec![10]]);
    }

    // --- Option<HashMap<K, V>> tests ---

    #[test]
    fn test_option_hashmap_into_some() {
        let source = SourceD {
            metadata: Some({
                let mut m = HashMap::new();
                m.insert("score".to_string(), 42u32);
                m
            }),
        };
        let target: TargetD = source.into();
        let meta = target.metadata.unwrap();
        assert_eq!(meta.get("score"), Some(&Score(42)));
    }

    #[test]
    fn test_option_hashmap_into_none() {
        let source = SourceD { metadata: None };
        let target: TargetD = source.into();
        assert_eq!(target.metadata, None);
    }

    #[test]
    fn test_option_hashmap_try_from() {
        let target = TargetD {
            metadata: Some({
                let mut m = HashMap::new();
                m.insert("level".to_string(), Score(99));
                m
            }),
        };
        let source = SourceD::try_from(target).unwrap();
        let meta = source.metadata.unwrap();
        assert_eq!(meta.get("level"), Some(&99));
    }

    // --- Option<Option<T>> tests ---

    #[test]
    fn test_option_option_into() {
        let source = SourceE {
            nested_opt: Some(Some(7)),
        };
        let target: TargetE = source.into();
        assert_eq!(target.nested_opt, Some(Some(Score(7))));

        let source = SourceE {
            nested_opt: Some(None),
        };
        let target: TargetE = source.into();
        assert_eq!(target.nested_opt, Some(None));

        let source = SourceE { nested_opt: None };
        let target: TargetE = source.into();
        assert_eq!(target.nested_opt, None);
    }

    #[test]
    fn test_option_option_try_from() {
        let target = TargetE {
            nested_opt: Some(Some(Score(3))),
        };
        let source = SourceE::try_from(target).unwrap();
        assert_eq!(source.nested_opt, Some(Some(3)));
    }

    // --- HashMap<K, Vec<V>> tests ---

    #[test]
    fn test_hashmap_vec_into() {
        let source = SourceF {
            grouped: {
                let mut m = HashMap::new();
                m.insert("a".to_string(), vec![1, 2, 3]);
                m
            },
        };
        let target: TargetF = source.into();
        assert_eq!(
            target.grouped.get("a"),
            Some(&vec![Score(1), Score(2), Score(3)])
        );
    }

    #[test]
    fn test_hashmap_vec_try_from() {
        let target = TargetF {
            grouped: {
                let mut m = HashMap::new();
                m.insert("x".to_string(), vec![Score(10)]);
                m
            },
        };
        let source = SourceF::try_from(target).unwrap();
        assert_eq!(source.grouped.get("x"), Some(&vec![10]));
    }

    // --- Unwrap on Option<Vec<T>> tests ---

    #[test]
    fn test_unwrap_option_vec_into() {
        let source = SourceG {
            items: Some(vec!["a".to_string(), "b".to_string()]),
        };
        let target: TargetG = source.into();
        assert_eq!(target.items, vec![Tag("a".to_string()), Tag("b".to_string())]);
    }
}

fn main() {
    println!("Running nested container conversion tests...");

    let source = SourceA {
        tags: Some(vec!["test".to_string()]),
    };
    let target: TargetA = source.into();
    println!("Option<Vec<T>> conversion: {:?}", target);

    let source = SourceB {
        scores: vec![Some(1), None, Some(3)],
    };
    let target: TargetB = source.into();
    println!("Vec<Option<T>> conversion: {:?}", target);

    let source = SourceC {
        matrix: vec![vec![1, 2], vec![3]],
    };
    let target: TargetC = source.into();
    println!("Vec<Vec<T>> conversion: {:?}", target);

    println!("All nested container tests passed!");
}
