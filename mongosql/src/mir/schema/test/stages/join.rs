use crate::{
    map,
    mir::{
        schema::{
            test::{
                test_document_a, test_document_b, test_document_c, TEST_DOCUMENT_SCHEMA_A,
                TEST_DOCUMENT_SCHEMA_B, TEST_DOCUMENT_SCHEMA_C, TEST_DOCUMENT_SCHEMA_S,
            },
            Error as mir_error, SchemaCache,
        },
        *,
    },
    schema::{Atomic, Document, ResultSet, Schema, BOOLEAN_OR_NULLISH},
    set, test_schema, unchecked_unique_linked_hash_map,
    util::{mir_field_path, mir_project_collection},
};
use agg_ast::definitions::Namespace;
use mongosql_datastructures::binding_tuple::DatasourceName::Bottom;

mod equijoin {
    use super::*;

    test_schema!(
        left_equijoin,
        expected = Ok(ResultSet {
            schema_env: map! {
                ("bar", 0u16).into() => Schema::AnyOf(set![
                            Schema::Missing,
                            TEST_DOCUMENT_SCHEMA_A.clone()
                    ]
                ),
                ("foo", 0u16).into() => Schema::AnyOf(set![
                        TEST_DOCUMENT_SCHEMA_A.clone()
                    ]
                ),
            },
            min_size: 1,
            max_size: None,
        }),
        input = Stage::MqlIntrinsic(MqlStage::EquiJoin(EquiJoin {
            join_type: JoinType::Left,
            source: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_a()],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            from: mir_project_collection(None, "bar", None, None),
            local_field: Box::new(mir_field_path("foo", vec!["a"])),
            foreign_field: Box::new(mir_field_path("bar", vec!["a"])),
            cache: SchemaCache::new(),
        })),
        catalog = Catalog::new(map! {
            Namespace {database: "test_db".into(), collection: "foo".into()} => TEST_DOCUMENT_SCHEMA_A.clone(),
            Namespace {database: "test_db".into(), collection: "bar".into()} => TEST_DOCUMENT_SCHEMA_A.clone(),
        }),
    );

    test_schema!(
        inner_equijoin,
        expected = Ok(ResultSet {
            schema_env: map! {
                ("bar", 0u16).into() => TEST_DOCUMENT_SCHEMA_A.clone(),
                ("foo", 0u16).into() => Schema::AnyOf(set![ TEST_DOCUMENT_SCHEMA_A.clone() ]),
            },
            min_size: 0,
            max_size: None,
        }),
        input = Stage::MqlIntrinsic(MqlStage::EquiJoin(EquiJoin {
            join_type: JoinType::Inner,
            source: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_a()],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            from: mir_project_collection(None, "bar", None, None),
            local_field: Box::new(mir_field_path("foo", vec!["a"])),
            foreign_field: Box::new(mir_field_path("bar", vec!["a"])),
            cache: SchemaCache::new(),
        })),
        catalog = Catalog::new(map! {
            Namespace {database: "test_db".into(), collection: "foo".into()} => TEST_DOCUMENT_SCHEMA_A.clone(),
            Namespace {database: "test_db".into(), collection: "bar".into()} => TEST_DOCUMENT_SCHEMA_A.clone(),
        }),
    );

    test_schema!(
        equijoin_fields_not_comparable,
        expected_error_code = 1005,
        expected = Err(schema::Error::InvalidComparison(
            "equijoin comparison",
            Schema::AnyOf(set![Schema::Atomic(Atomic::Integer),]).into(),
            Schema::Atomic(Atomic::String).into(),
        )),
        input = Stage::MqlIntrinsic(MqlStage::EquiJoin(EquiJoin {
            join_type: JoinType::Inner,
            source: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_a()],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            from: mir_project_collection(None, "bar", None, None),
            local_field: Box::new(mir_field_path("foo", vec!["a"])),
            foreign_field: Box::new(mir_field_path("bar", vec!["s"])),
            cache: SchemaCache::new(),
        })),
        catalog = Catalog::new(map! {
            Namespace {database: "test_db".into(), collection: "foo".into()} => TEST_DOCUMENT_SCHEMA_A.clone(),
            Namespace {database: "test_db".into(), collection: "bar".into()} => TEST_DOCUMENT_SCHEMA_S.clone(),
        }),
    );
}

mod lateral {
    use super::*;

    test_schema!(
        left_lateral_join,
        expected = Ok(ResultSet {
            schema_env: map! {
                ("bar", 0u16).into() => Schema::AnyOf(set![
                            Schema::Missing,
                            TEST_DOCUMENT_SCHEMA_A.clone()
                    ]
                ),
                ("foo", 0u16).into() => Schema::AnyOf(set![
                        TEST_DOCUMENT_SCHEMA_A.clone()
                    ]
                ),
            },
            min_size: 1,
            max_size: None,
        }),
        input = Stage::MqlIntrinsic(MqlStage::LateralJoin(LateralJoin {
            join_type: JoinType::Left,
            source: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_a()],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            subquery: mir_project_collection(None, "bar", None, None),
            cache: SchemaCache::new(),
        })),
        catalog = Catalog::new(map! {
            Namespace {database: "test_db".into(), collection: "foo".into()} => TEST_DOCUMENT_SCHEMA_A.clone(),
            Namespace {database: "test_db".into(), collection: "bar".into()} => TEST_DOCUMENT_SCHEMA_A.clone(),
        }),
    );

    test_schema!(
        inner_lateral_join,
        expected = Ok(ResultSet {
            schema_env: map! {
                ("bar", 0u16).into() => TEST_DOCUMENT_SCHEMA_A.clone(),
                ("foo", 0u16).into() => Schema::AnyOf(set![ TEST_DOCUMENT_SCHEMA_A.clone() ]),
            },
            min_size: 0,
            max_size: None,
        }),
        input = Stage::MqlIntrinsic(MqlStage::LateralJoin(LateralJoin {
            join_type: JoinType::Inner,
            source: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_a()],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            subquery: mir_project_collection(None, "bar", None, None),
            cache: SchemaCache::new(),
        })),
        catalog = Catalog::new(map! {
            Namespace {database: "test_db".into(), collection: "foo".into()} => TEST_DOCUMENT_SCHEMA_A.clone(),
            Namespace {database: "test_db".into(), collection: "bar".into()} => TEST_DOCUMENT_SCHEMA_A.clone(),
        }),
    );
}

mod standard {
    use super::*;

    test_schema!(
        left_join,
        expected = Ok(ResultSet {
            schema_env: map! {
                ("bar", 0u16).into() => Schema::AnyOf(set![
                        Schema::Missing,
                        Schema::AnyOf(set![
                                TEST_DOCUMENT_SCHEMA_B.clone()
                            ]
                        ),
                    ]
                ),
                ("foo", 0u16).into() => Schema::AnyOf(set![
                        TEST_DOCUMENT_SCHEMA_A.clone()
                    ]
                ),
            },
            min_size: 1,
            max_size: Some(1),
        }),
        input = Stage::Join(Join {
            join_type: JoinType::Left,
            left: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_a()],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            right: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_b()],
                alias: "bar".into(),
                cache: SchemaCache::new(),
            })),
            condition: Some(Expression::Literal(LiteralValue::Boolean(false))),
            cache: SchemaCache::new(),
        }),
    );

    test_schema!(
        cross_join,
        expected_pat = Ok(ResultSet {
            min_size: 6,
            max_size: Some(6),
            ..
        }),
        input = Stage::Join(Join {
            join_type: JoinType::Inner,
            left: Box::new(Stage::Array(ArraySource {
                array: vec![
                    Expression::Document(
                        unchecked_unique_linked_hash_map! {
                            "a".into() => Expression::Literal(LiteralValue::Integer(1))
                        }
                        .into()
                    ),
                    Expression::Document(
                        unchecked_unique_linked_hash_map! {
                            "a".into() => Expression::Literal(LiteralValue::Integer(2))
                        }
                        .into()
                    ),
                    Expression::Document(
                        unchecked_unique_linked_hash_map! {
                            "a".into() => Expression::Literal(LiteralValue::Integer(3))
                        }
                        .into()
                    )
                ],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            right: Box::new(Stage::Array(ArraySource {
                array: vec![
                    Expression::Document(
                        unchecked_unique_linked_hash_map! {
                            "b".into() => Expression::Literal(LiteralValue::Integer(5))
                        }
                        .into()
                    ),
                    Expression::Document(
                        unchecked_unique_linked_hash_map! {
                            "b".into() => Expression::Literal(LiteralValue::Integer(6))
                        }
                        .into()
                    ),
                ],
                alias: "bar".into(),
                cache: SchemaCache::new(),
            })),
            condition: None,
            cache: SchemaCache::new(),
        }),
    );

    test_schema!(
        inner_join,
        expected_pat = Ok(ResultSet {
            min_size: 0,
            max_size: Some(6),
            ..
        }),
        input = Stage::Join(Join {
            join_type: JoinType::Inner,
            left: Box::new(Stage::Array(ArraySource {
                array: vec![
                    Expression::Document(
                        unchecked_unique_linked_hash_map! {
                            "a".into() => Expression::Literal(LiteralValue::Integer(1))
                        }
                        .into()
                    ),
                    Expression::Document(
                        unchecked_unique_linked_hash_map! {
                            "a".into() => Expression::Literal(LiteralValue::Integer(2))
                        }
                        .into()
                    ),
                    Expression::Document(
                        unchecked_unique_linked_hash_map! {
                            "a".into() => Expression::Literal(LiteralValue::Integer(3))
                        }
                        .into()
                    )
                ],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            right: Box::new(Stage::Array(ArraySource {
                array: vec![
                    Expression::Document(
                        unchecked_unique_linked_hash_map! {
                            "b".into() => Expression::Literal(LiteralValue::Integer(5))
                        }
                        .into()
                    ),
                    Expression::Document(
                        unchecked_unique_linked_hash_map! {
                            "b".into() => Expression::Literal(LiteralValue::Integer(6))
                        }
                        .into()
                    ),
                ],
                alias: "bar".into(),
                cache: SchemaCache::new(),
            })),
            condition: Some(Expression::Literal(LiteralValue::Boolean(false))),
            cache: SchemaCache::new(),
        }),
    );

    test_schema!(
        inner_and_left_join,
        expected = Ok(ResultSet {
            schema_env: map! {
                ("foo", 0u16).into() => Schema::AnyOf(set![
                        TEST_DOCUMENT_SCHEMA_A.clone(),
                    ]
                ),
                ("bar", 0u16).into() => Schema::AnyOf(set![
                        TEST_DOCUMENT_SCHEMA_B.clone(),
                    ]
                ),
                ("car", 0u16).into() => Schema::AnyOf(set![
                        Schema::Missing,
                        Schema::AnyOf(set![TEST_DOCUMENT_SCHEMA_C.clone()]),
                    ]
                ),
            },
            min_size: 1,
            max_size: Some(1),
        }),
        input = Stage::Join(Join {
            join_type: JoinType::Inner,
            left: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_a()],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            right: Box::new(Stage::Join(Join {
                join_type: JoinType::Left,
                left: Box::new(Stage::Array(ArraySource {
                    array: vec![test_document_b()],
                    alias: "bar".into(),
                    cache: SchemaCache::new(),
                })),
                right: Box::new(Stage::Array(ArraySource {
                    array: vec![test_document_c()],
                    alias: "car".into(),
                    cache: SchemaCache::new(),
                })),
                condition: Some(Expression::Literal(LiteralValue::Boolean(false))),
                cache: SchemaCache::new(),
            })),
            condition: None,
            cache: SchemaCache::new(),
        }),
    );

    test_schema!(
        invalid_join_condition,
        expected_error_code = 1002,
        expected = Err(mir_error::SchemaChecking {
            name: "join condition",
            required: BOOLEAN_OR_NULLISH.clone().into(),
            found: Schema::Atomic(Atomic::Integer).into(),
        }),
        input = Stage::Join(Join {
            join_type: JoinType::Left,
            left: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_a()],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            right: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_b()],
                alias: "bar".into(),
                cache: SchemaCache::new(),
            })),
            condition: Some(Expression::Literal(LiteralValue::Integer(5))),
            cache: SchemaCache::new(),
        }),
    );

    test_schema!(
        join_condition_uses_left_datasource,
        expected_pat = Ok(ResultSet { .. }),
        input = Stage::Join(Join {
            join_type: JoinType::Left,
            left: Box::new(Stage::Array(ArraySource {
                array: vec![Expression::Document(
                    unchecked_unique_linked_hash_map! {
                        "a".into() => Expression::Literal(LiteralValue::Boolean(true))
                    }
                    .into()
                )],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            right: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_b()],
                alias: "bar".into(),
                cache: SchemaCache::new(),
            })),
            condition: Some(Expression::TypeAssertion(TypeAssertionExpr {
                expr: Box::new(Expression::FieldAccess(FieldAccess::new(
                    Box::new(Expression::Reference(("foo", 0u16).into())),
                    "a".to_string(),
                ))),
                target_type: Type::Boolean,
            })),
            cache: SchemaCache::new(),
        }),
    );

    test_schema!(
        join_condition_uses_right_datasource,
        expected_pat = Ok(ResultSet { .. }),
        input = Stage::Join(Join {
            join_type: JoinType::Left,
            left: Box::new(Stage::Array(ArraySource {
                array: vec![test_document_a()],
                alias: "foo".into(),
                cache: SchemaCache::new(),
            })),
            right: Box::new(Stage::Array(ArraySource {
                array: vec![Expression::Document(
                    unchecked_unique_linked_hash_map! {
                        "b".into() => Expression::Literal(LiteralValue::Boolean(true))
                    }
                    .into()
                )],
                alias: "bar".into(),
                cache: SchemaCache::new(),
            })),
            condition: Some(Expression::TypeAssertion(TypeAssertionExpr {
                expr: Box::new(Expression::FieldAccess(FieldAccess::new(
                    Box::new(Expression::Reference(("bar", 0u16).into())),
                    "b".to_string(),
                ))),
                target_type: Type::Boolean,
            })),
            cache: SchemaCache::new(),
        }),
    );

    test_schema!(
        join_condition_uses_correlated_datasource,
        expected = Ok(Schema::Atomic(Atomic::Boolean)),
        input = Expression::Subquery(SubqueryExpr {
            output_expr: Box::new(Expression::FieldAccess(FieldAccess::new(
                Box::new(Expression::Reference((Bottom, 1u16).into())),
                "a".into(),
            ))),
            subquery: Box::new(Stage::Project(Project {
                is_add_fields: false,
                source: Box::new(Stage::Join(Join {
                    join_type: JoinType::Left,
                    left: Box::new(Stage::Array(ArraySource {
                        array: vec![test_document_b()],
                        alias: "bar".into(),
                        cache: SchemaCache::new(),
                    })),
                    right: Box::new(Stage::Array(ArraySource {
                        array: vec![test_document_c()],
                        alias: "car".into(),
                        cache: SchemaCache::new(),
                    })),
                    condition: Some(Expression::TypeAssertion(TypeAssertionExpr {
                        expr: Box::new(Expression::FieldAccess(FieldAccess::new(
                            Box::new(Expression::Reference(("foo", 0u16).into())),
                            "a".to_string(),
                        ))),
                        target_type: Type::Boolean,
                    })),
                    cache: SchemaCache::new(),
                }),),
                expression: map! {
                    (Bottom, 1u16).into() => Expression::Document(unchecked_unique_linked_hash_map! {
                        "a".into() => Expression::FieldAccess(FieldAccess{
                            expr: Box::new(Expression::Reference(("foo", 0u16).into())),
                            field: "a".into(),
                            is_nullable: true,
                        })
                    }.into())
                },
                cache: SchemaCache::new(),
            })),
            is_nullable: true,
        }),
        schema_env = map! {
            ("foo", 0u16).into() => Schema::Document( Document{
                keys: map! {
                    "a".into() => Schema::Atomic(Atomic::Boolean)
                },
                required: set! { "a".into() },
                additional_properties: false,
                ..Default::default()
                }),
        },
    );
}
