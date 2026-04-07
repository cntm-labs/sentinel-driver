use sentinel_driver::pipeline::batch::PipelineBatch;

#[test]
fn test_batch_empty() {
    let batch = PipelineBatch::new();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
}

#[test]
fn test_batch_add_queries() {
    let mut batch = PipelineBatch::new();
    batch.add("SELECT 1", vec![], vec![]);
    batch.add("SELECT 2", vec![], vec![]);
    assert_eq!(batch.len(), 2);
    assert!(!batch.is_empty());
}
