/*!
This module tests the following properties:

1. The `last` term matching strategy starts removing terms from the query
starting from the end if no more results match it.
2. Phrases are never deleted by the `last` term matching strategy
3. Duplicate words don't affect the ranking of a document according to the `words` ranking rule
4. The proximity of the first and last word of a phrase to its adjacent terms is taken into
account by the proximity ranking rule.
5. Unclosed double quotes still make a phrase
6. The `all` term matching strategy does not remove any term from the query
7. The search is capable of returning no results if no documents match the query
*/

use crate::{index::tests::TempIndex, Criterion, Search, SearchResult, TermsMatchingStrategy};

fn create_quick_brown_fox_trivial_index() -> TempIndex {
    let index = TempIndex::new();

    index
        .update_settings(|s| {
            s.set_primary_key("id".to_owned());
            s.set_searchable_fields(vec!["text".to_owned()]);
            s.set_criteria(vec![Criterion::Words]);
        })
        .unwrap();

    index
        .add_documents(documents!([
            {
                "id": 0,
                "text": "",
            },
            {
                "id": 1,
                "text": "the",
            },
            {
                "id": 2,
                "text": "the quick",
            },
            {
                "id": 3,
                "text": "the quick brown",
            },
            {
                "id": 4,
                "text": "the quick brown fox",
            },
            {
                "id": 5,
                "text": "the quick brown fox jumps",
            },
            {
                "id": 6,
                "text": "the quick brown fox jumps over",
            },
            {
                "id": 7,
                "text": "the quick brown fox jumps over the",
            },
            {
                "id": 8,
                "text": "the quick brown fox jumps over the lazy",
            },
            {
                "id": 9,
                "text": "the quick brown fox jumps over the lazy dog",
            },
            {
                "id": 10,
                "text": "the brown quick fox jumps over the lazy dog",
            },
            {
                "id": 11,
                "text": "the quick brown fox talks to the lazy and slow dog",
            },
            {
                "id": 12,
                "text": "the quick brown fox talks to the lazy dog",
            },
            {
                "id": 13,
                "text": "the mighty and quick brown fox jumps over the lazy dog",
            },
            {
                "id": 14,
                "text": "the great quick brown fox jumps over the lazy dog",
            },
            {
                "id": 15,
                "text": "this quick brown and very scary fox jumps over the lazy dog",
            },
            {
                "id": 16,
                "text": "this quick brown and scary fox jumps over the lazy dog",
            },
            {
                "id": 17,
                "text": "the quick brown fox jumps over the really lazy dog",
            },
            {
                "id": 18,
                "text": "the brown quick fox jumps over the really lazy dog",
            },
            {
                "id": 19,
                "text": "the brown quick fox immediately jumps over the really lazy dog",
            },
            {
                "id": 20,
                "text": "the brown quick fox immediately jumps over the really lazy blue dog",
            },
            {
                "id": 21,
                "text": "the quick brown. quick brown fox. brown fox jumps. fox jumps over. over the lazy. the lazy dog.",
            },
            {
                "id": 22,
                "text": "the, quick, brown, fox, jumps, over, the, lazy, dog",
            }
        ]))
        .unwrap();
    index
}

#[test]
fn test_words_tms_last_simple() {
    let index = create_quick_brown_fox_trivial_index();

    let txn = index.read_txn().unwrap();
    let mut s = Search::new(&txn, &index);
    s.query("the quick brown fox jumps over the lazy dog");
    s.terms_matching_strategy(TermsMatchingStrategy::Last);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();

    // 6 and 7 have the same score because "the" appears twice
    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[9, 10, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 8, 6, 7, 5, 4, 11, 12, 3]");

    let mut s = Search::new(&txn, &index);
    s.query("extravagant the quick brown fox jumps over the lazy dog");
    s.terms_matching_strategy(TermsMatchingStrategy::Last);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[]");
}

#[test]
fn test_words_tms_last_phrase() {
    let index = create_quick_brown_fox_trivial_index();

    let txn = index.read_txn().unwrap();
    let mut s = Search::new(&txn, &index);
    s.query("\"the quick brown fox\" jumps over the lazy dog");
    s.terms_matching_strategy(TermsMatchingStrategy::Last);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();

    // "The quick brown fox" is a phrase, not deleted by this term matching strategy
    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[9, 17, 21, 8, 6, 7, 5, 4, 11, 12]");

    let mut s = Search::new(&txn, &index);
    s.query("\"the quick brown fox\" jumps over the \"lazy\" dog");
    s.terms_matching_strategy(TermsMatchingStrategy::Last);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();

    // "lazy" is a phrase, not deleted by this term matching strategy
    // but words before it can be deleted
    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[9, 17, 21, 8, 11, 12]");

    let mut s = Search::new(&txn, &index);
    s.query("\"the quick brown fox jumps over the lazy dog\"");
    s.terms_matching_strategy(TermsMatchingStrategy::Last);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();

    // The whole query is a phrase, no terms are removed
    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[9]");

    let mut s = Search::new(&txn, &index);
    s.query("\"the quick brown fox jumps over the lazy dog");
    s.terms_matching_strategy(TermsMatchingStrategy::Last);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();

    // The whole query is still a phrase, even without closing quotes, so no terms are removed
    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[9]");
}

#[test]
fn test_words_proximity_tms_last_simple() {
    let index = create_quick_brown_fox_trivial_index();
    index
        .update_settings(|s| {
            s.set_criteria(vec![Criterion::Words, Criterion::Proximity]);
        })
        .unwrap();

    let txn = index.read_txn().unwrap();
    let mut s = Search::new(&txn, &index);
    s.query("the quick brown fox jumps over the lazy dog");
    s.terms_matching_strategy(TermsMatchingStrategy::Last);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();

    // 7 is better than 6 because of the proximity between "the" and its surrounding terms
    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[9, 21, 14, 17, 13, 10, 18, 19, 20, 16, 15, 22, 8, 7, 6, 5, 4, 11, 12, 3]");

    let mut s = Search::new(&txn, &index);
    s.query("the brown quick fox jumps over the lazy dog");
    s.terms_matching_strategy(TermsMatchingStrategy::Last);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();

    // 10 is better than 9 because of the proximity between "quick" and "brown"
    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[10, 18, 19, 9, 20, 21, 14, 17, 13, 16, 15, 22, 8, 7, 6, 5, 4, 11, 12, 3]");
}

#[test]
fn test_words_proximity_tms_last_phrase() {
    let index = create_quick_brown_fox_trivial_index();
    index
        .update_settings(|s| {
            s.set_criteria(vec![Criterion::Words, Criterion::Proximity]);
        })
        .unwrap();

    let txn = index.read_txn().unwrap();
    let mut s = Search::new(&txn, &index);
    s.query("the \"quick brown\" fox jumps over the lazy dog");
    s.terms_matching_strategy(TermsMatchingStrategy::Last);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();

    // "quick brown" is a phrase. The proximity of its first and last words
    // to their adjacent query words should be taken into account
    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[9, 21, 14, 17, 13, 16, 15, 8, 7, 6, 5, 4, 11, 12, 3]");

    let mut s = Search::new(&txn, &index);
    s.query("the \"quick brown\" \"fox jumps\" over the lazy dog");
    s.terms_matching_strategy(TermsMatchingStrategy::Last);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();

    // "quick brown" is a phrase. The proximity of its first and last words
    // to their adjacent query words should be taken into account.
    // The same applies to `fox jumps`.
    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[9, 21, 14, 17, 13, 16, 15, 8, 7, 6, 5]");
}

#[test]
fn test_words_tms_all() {
    let index = create_quick_brown_fox_trivial_index();
    index
        .update_settings(|s| {
            s.set_criteria(vec![Criterion::Words, Criterion::Proximity]);
        })
        .unwrap();

    let txn = index.read_txn().unwrap();
    let mut s = Search::new(&txn, &index);
    s.query("the quick brown fox jumps over the lazy dog");
    s.terms_matching_strategy(TermsMatchingStrategy::All);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();

    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[9, 21, 14, 17, 13, 10, 18, 19, 20, 16, 15, 22]");

    let mut s = Search::new(&txn, &index);
    s.query("extravagant");
    s.terms_matching_strategy(TermsMatchingStrategy::All);
    let SearchResult { documents_ids, .. } = s.execute().unwrap();

    insta::assert_snapshot!(format!("{documents_ids:?}"), @"[]");
}