use sentinel_driver::protocol::backend::FieldDescription;
use sentinel_driver::statement::Statement;
use sentinel_driver::types::Oid;

#[test]
fn test_statement_no_params_no_columns() {
    let stmt = Statement::new(
        "s1".to_string(),
        "CREATE TABLE foo (id int)".to_string(),
        vec![],
        None,
    );

    assert_eq!(stmt.name(), "s1");
    assert_eq!(stmt.sql(), "CREATE TABLE foo (id int)");
    assert_eq!(stmt.param_count(), 0);
    assert_eq!(stmt.column_count(), 0);
    assert!(stmt.columns().is_none());
}

#[test]
fn test_statement_with_params_and_columns() {
    let stmt = Statement::new(
        "s2".to_string(),
        "SELECT name FROM users WHERE id = $1".to_string(),
        vec![Oid::INT4],
        Some(vec![FieldDescription {
            name: "name".to_string(),
            table_oid: 16384,
            column_id: 2,
            type_oid: 25,
            type_size: -1,
            type_modifier: -1,
            format: 1,
        }]),
    );

    assert_eq!(stmt.param_count(), 1);
    assert_eq!(stmt.param_types()[0], Oid::INT4);
    assert_eq!(stmt.column_count(), 1);
    assert_eq!(stmt.columns().unwrap()[0].name, "name");
}
