#![no_main]
use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use milli::heed::EnvOpenOptions;
use milli::update::{IndexDocuments, IndexDocumentsConfig, IndexerConfig};
use milli::Index;
use serde_json::{json, Value};
use tempfile::TempDir;

#[derive(Debug)]
enum Document {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}

impl<'a> Arbitrary<'a> for Document {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, arbitrary::Error> {
        use Document::*;

        let variant: u8 = u.arbitrary()?;
        let doc = match variant % 6 {
            0 => One,
            1 => Two,
            2 => Three,
            3 => Four,
            4 => Five,
            5 => Six,
            _ => unreachable!(),
        };
        Ok(doc)
    }
}

impl Document {
    pub fn to_d(&self) -> Value {
        match self {
            Document::One => json!({ "id": 0, "doggo": "bernese" }),
            Document::Two => json!({ "id": 0, "doggo": "golden" }),
            Document::Three => json!({ "id": 0, "catto": "jorts" }),
            Document::Four => json!({ "id": 1, "doggo": "bernese" }),
            Document::Five => json!({ "id": 1, "doggo": "golden" }),
            Document::Six => json!({ "id": 1, "catto": "jorts" }),
        }
    }
}

#[derive(Debug)]
enum DocId {
    Zero,
    One,
}

impl<'a> Arbitrary<'a> for DocId {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, arbitrary::Error> {
        use DocId::*;

        let variant: u8 = u.arbitrary()?;
        let doc = match variant % 2 {
            0 => Zero,
            1 => One,
            _ => unreachable!(),
        };
        Ok(doc)
    }
}

impl DocId {
    pub fn to_s(&self) -> String {
        match self {
            DocId::Zero => "0".to_string(),
            DocId::One => "1".to_string(),
        }
    }
}

#[derive(Debug)]
enum Operation {
    AddDoc(Document),
    DeleteDoc(DocId),
}

impl<'a> Arbitrary<'a> for Operation {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, arbitrary::Error> {
        use Operation::*;

        let variant: u8 = u.arbitrary()?;
        let op = match variant % 2 {
            0 => AddDoc(u.arbitrary()?),
            1 => DeleteDoc(u.arbitrary()?),
            _ => unreachable!(),
        };
        Ok(op)
    }
}

#[derive(Debug)]
struct Batch(Vec<Operation>);

impl<'a> Arbitrary<'a> for Batch {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, arbitrary::Error> {
        let batch = u.arbitrary_iter()?.collect::<Result<Vec<Operation>, _>>()?;
        Ok(Batch(batch))
    }
}

fuzz_target!(|batches: Vec<Batch>| {
    let mut options = EnvOpenOptions::new();
    options.map_size(1024 * 1024 * 1024 * 1024);
    let _tempdir = TempDir::new_in(".").unwrap();
    let index = Index::new(options, _tempdir.path()).unwrap();
    let indexer_config = IndexerConfig::default();
    let index_documents_config = IndexDocumentsConfig::default();

    for batch in batches {
        let mut wtxn = index.write_txn().unwrap();

        let mut builder = IndexDocuments::new(
            &mut wtxn,
            &index,
            &indexer_config,
            index_documents_config.clone(),
            |_| (),
            || false,
        )
        .unwrap();

        for op in batch.0 {
            match op {
                Operation::AddDoc(doc) => {
                    let documents = milli::documents::objects_from_json_value(doc.to_d());
                    let documents =
                        milli::documents::documents_batch_reader_from_objects(documents);
                    let (b, _added) = builder.add_documents(documents).unwrap();
                    builder = b;
                }
                Operation::DeleteDoc(id) => {
                    let (b, _removed) = builder.remove_documents(vec![id.to_s()]).unwrap();
                    builder = b;
                }
            }
        }
        builder.execute().unwrap();
        wtxn.commit().unwrap();

        // after executing a batch we check if the database is corrupted
        let rtxn = index.read_txn().unwrap();
        let res = index.search(&rtxn).execute().unwrap();
        index.documents(&rtxn, res.documents_ids).unwrap();
    }
});
