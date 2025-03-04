package mongosql_test

import (
	"reflect"
	"strings"
	"testing"

	"github.com/google/go-cmp/cmp"
	"github.com/mongodb/mongosql/go/mongosql"
	"github.com/mongodb/mongosql/go/mongosql/internal/util"
	"go.mongodb.org/mongo-driver/bson"
	"go.mongodb.org/mongo-driver/bson/bsontype"
	"go.mongodb.org/mongo-driver/x/bsonx/bsoncore"
)

func TestVersion(t *testing.T) {
	v := mongosql.Version()

	parts := strings.SplitN(v, "-", 2)

	release := parts[0]
	releaseParts := strings.Split(release, ".")
	if len(releaseParts) != 3 {
		t.Fatalf("expected version %q to have three release parts", v)
	}

	if len(parts) == 2 {
		preRelease := parts[1]
		if len(preRelease) < 1 {
			t.Fatalf("expected version %q to have non-empty pre-release", v)
		}
	}
}

func TestTranslate(t *testing.T) {
	schema, err := util.GenerateDefaultCollectionSchema()
	if err != nil {
		t.Fatalf("expected err to be nil, got '%s'", err)
	}

	catalogSchema := map[string]map[string]bsoncore.Document{
		"bar": {"foo": schema},
	}

	translation, err := mongosql.Translate(mongosql.TranslationArgs{
		DB:            "bar",
		SQL:           "select * from foo",
		CatalogSchema: catalogSchema,
	})
	if err != nil {
		t.Fatalf("expected err to be nil, got '%s'", err)
	}

	if translation.TargetDB != "bar" {
		t.Fatalf("expected targetDB to be 'bar', got '%s'", translation.TargetDB)
	}

	if translation.TargetCollection != "foo" {
		t.Fatalf("expected targetCollection to be 'foo', got '%s'", translation.TargetCollection)
	}

	var pipeline []bson.D
	val := bson.RawValue{
		Type:  bsontype.Array,
		Value: translation.Pipeline,
	}
	err = val.Unmarshal(&pipeline)
	if err != nil {
		t.Fatalf("expected pipeline to unmarshal into []bson.D, but failed: %s", err)
	}

	if len(pipeline) != 1 {
		t.Fatalf("expected pipeline to have one stage, but found %d", len(pipeline))
	}

	expectedStage0 := bson.D{
		{"$project", bson.D{
			{"foo", "$$ROOT"},
			{"_id", int32(0)},
		}},
	}
	if !reflect.DeepEqual(expectedStage0, pipeline[0]) {
		t.Fatalf("expected stages to be equal, but they weren't:\n%s\nand\n%s", expectedStage0, pipeline[0])
	}

	expectedResultSetSchema := bson.D{
		{"bsonType", "object"},
		{"properties", bson.D{
			{"foo", bson.D{
				{"bsonType", "object"},
				{"properties", bson.D{}},
				{"additionalProperties", true},
			}},
		}},
		{"required", bson.A{"foo"}},
		{"additionalProperties", false},
	}

	util.CheckResultSetSchema(t, expectedResultSetSchema, translation.ResultSetSchema)
}

func TestTranslateError(t *testing.T) {
	_, err := mongosql.Translate(mongosql.TranslationArgs{
		DB:            "bar",
		SQL:           "notavalidquery",
		CatalogSchema: nil,
	})

	if err == nil {
		t.Fatalf("expected error to be non-nil, but it was nil")
	}

	tErr, ok := err.(mongosql.TranslationError)
	if !ok {
		t.Fatalf("expected error to be a TranslationError, but it wasn't")
	}

	if tErr.IsInternal() {
		t.Fatalf("semantic translation errors should be external, but an internal error was found")
	}

	if !strings.Contains(err.Error(), "parse error: Error 2001: Unrecognized token `notavalidquery`, expected: `SELECT`") {
		t.Fatalf("error message did not contain expected text: %q", err.Error())
	}
}

func TestTranslatePanic(t *testing.T) {
	_, err := mongosql.Translate(mongosql.TranslationArgs{
		DB:            "__test_panic",
		SQL:           "__test_panic",
		CatalogSchema: nil,
	})

	if err == nil {
		t.Fatalf("expected error to be non-nil, but it was nil")
	}

	tErr, ok := err.(mongosql.TranslationError)
	if !ok {
		t.Fatalf("expected error to be a TranslationError, but it wasn't")
	}

	if !tErr.IsInternal() {
		t.Fatalf("an error from a caught panic should be internal, but an external error was found")
	}

	if !strings.Contains(err.Error(), "Internal Error: report this to MongoDB: panic thrown") {
		t.Fatalf("error message did not contain expected text: %q", err.Error())
	}
}

func TestDatabaseResultNotEmpty(t *testing.T) {
	translation, err := mongosql.Translate(mongosql.TranslationArgs{
		DB:            "test",
		SQL:           "select 1",
		CatalogSchema: nil,
	})

	if err != nil {
		t.Fatalf("expected err to be nil, got '%s'", err)
	}

	if translation.TargetDB == "" {
		t.Fatalf("expected non-empty TargetDB, but it was empty")
	}
}

func TestCatalogSchema(t *testing.T) {
	schema := bson.M{
		"bsonType": "object",
		"properties": bson.M{
			"a": bson.M{"bsonType": "double"},
		},
	}

	bytes, err := bson.Marshal(&schema)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	catalogSchema := map[string]map[string]bsoncore.Document{
		"bar": {"foo": bsoncore.Document(bytes)},
	}

	translation, err := mongosql.Translate(mongosql.TranslationArgs{
		DB:            "bar",
		SQL:           "select * from foo",
		CatalogSchema: catalogSchema,
	})
	if err != nil {
		t.Fatalf("expected err to be nil, got '%s'", err)
	}

	expectedResultSetSchema := bson.D{
		{"bsonType", "object"},
		{"properties", bson.D{
			{"foo", bson.D{
				{"bsonType", "object"},
				{"properties", bson.D{
					{"a", bson.D{{"bsonType", "double"}}},
				}},
				{"additionalProperties", true},
			}},
		}},
		{"required", bson.A{"foo"}},
		{"additionalProperties", false},
	}

	util.CheckResultSetSchema(t, expectedResultSetSchema, translation.ResultSetSchema)

	expectedSelectOrder := bson.A{
		bson.A{"foo", "a"},
	}
	util.CheckSelectListOrder(t, expectedSelectOrder, translation.SelectOrder)
}

func TestCatalogSchemaMultipleCollections(t *testing.T) {
	barBazSchema, err := util.GenerateTestSchema()
	if err != nil {
		t.Fatalf("expected err to be nil, got '%s'", err)
	}

	fooSchema, err := util.GenerateDefaultCollectionSchema()
	if err != nil {
		t.Fatalf("expected err to be nil, got '%s'", err)
	}

	catalogSchema := map[string]map[string]bsoncore.Document{
		"foo": {"bar": barBazSchema, "baz": barBazSchema},
		"bar": {"foo": fooSchema},
	}

	translation, err := mongosql.Translate(mongosql.TranslationArgs{
		DB:            "bar",
		SQL:           "select * from foo",
		CatalogSchema: catalogSchema,
	})
	if err != nil {
		t.Fatalf("expected err to be nil, got '%s'", err)
	}

	expectedResultSetSchema := bson.D{
		{"bsonType", "object"},
		{"properties", bson.D{
			{"foo", bson.D{
				{"bsonType", "object"},
				{"properties", bson.D{}},
				{"additionalProperties", true},
			}},
		}},
		{"required", bson.A{"foo"}},
		{"additionalProperties", false},
	}

	util.CheckResultSetSchema(t, expectedResultSetSchema, translation.ResultSetSchema)
}

func TestCatalogSchemaMultipleNamespaces(t *testing.T) {
	schema, err := util.GenerateTestSchema()
	if err != nil {
		if err != nil {
			t.Fatalf("expected err to be nil, got '%s'", err)
		}
	}

	catalogSchema := map[string]map[string]bsoncore.Document{
		"bar": {"foo": schema},
		"baz": {"foo": schema},
	}

	translation, err := mongosql.Translate(mongosql.TranslationArgs{
		DB:            "bar",
		SQL:           "select * from foo",
		CatalogSchema: catalogSchema,
	})
	if err != nil {
		t.Fatalf("expected err to be nil, got '%s'", err)
	}

	expectedResultSetSchema := bson.D{
		{"bsonType", "object"},
		{"properties", bson.D{
			{"foo", bson.D{
				{"bsonType", "object"},
				{"properties", bson.D{
					{"a", bson.D{{"bsonType", "double"}}},
				}},
				{"additionalProperties", true},
			}},
		}},
		{"required", bson.A{"foo"}},
		{"additionalProperties", false},
	}

	util.CheckResultSetSchema(t, expectedResultSetSchema, translation.ResultSetSchema)
}

func TestCatalogSchemaEmpty(t *testing.T) {
	catalogSchema := map[string]map[string]bsoncore.Document{}

	_, err := mongosql.Translate(mongosql.TranslationArgs{
		DB:            "bar",
		SQL:           "select * from foo",
		CatalogSchema: catalogSchema,
	})

	if err == nil {
		t.Fatalf("expected error to be non-nil, but it was nil")
	}

	tErr, ok := err.(mongosql.TranslationError)
	if !ok {
		t.Fatalf("expected error to be a TranslationError, but it wasn't")
	}

	if tErr.IsInternal() {
		t.Fatalf("semantic translation errors should be external, but an internal error was found")
	}

	if !strings.Contains(err.Error(), "algebrize error: Error 1016: unknown collection 'foo' in database 'bar'") {
		t.Fatalf("error message did not contain expected text: %q", err.Error())
	}
}

// TestSelectOrderMultipleElements verifies that multiple, non lexicographic
// select values are properly parsed in the select order
func TestSelectOrderMultipleElements(t *testing.T) {
	schema := bson.M{
		"bsonType": "object",
		"properties": bson.M{
			"a": bson.M{"bsonType": "int"},
			"b": bson.M{"bsonType": "int"},
			"c": bson.M{"bsonType": "int"},
		},
	}
	bytes, err := bson.Marshal(&schema)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	catalogSchema := map[string]map[string]bsoncore.Document{
		"bar": {"foo": bsoncore.Document(bytes)},
	}

	translation, err := mongosql.Translate(mongosql.TranslationArgs{
		DB:            "bar",
		SQL:           "select c, a, b from foo",
		CatalogSchema: catalogSchema,
	})
	if err != nil {
		t.Fatalf("expected err to be nil, got '%s'", err)
	}

	expectedSelectOrder := bson.A{
		bson.A{"", "c"},
		bson.A{"", "a"},
		bson.A{"", "b"},
	}

	util.CheckSelectListOrder(t, expectedSelectOrder, translation.SelectOrder)
}

// TestArrayStyleItems verifies that translation succeeds with a
// catalog schema containing an array-style `items` field.
func TestArrayStyleItems(t *testing.T) {
	schema := bson.M{
		"bsonType": "object",
		"properties": bson.M{
			"a": bson.M{
				"bsonType": "array",
				"items":    []bson.M{{"bsonType": "int"}},
			},
		},
	}

	bytes, err := bson.Marshal(&schema)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	catalogSchema := map[string]map[string]bsoncore.Document{
		"bar": {"foo": bsoncore.Document(bytes)},
	}

	_, err = mongosql.Translate(mongosql.TranslationArgs{
		DB:            "bar",
		SQL:           "select * from foo",
		CatalogSchema: catalogSchema,
	})
	if err != nil {
		t.Fatalf("expected err to be nil, got '%s'", err)
	}
}

func TestGetNamespaces(t *testing.T) {
	tests := []struct {
		name               string
		db                 string
		sql                string
		skipReason         string
		expectedError      bool
		expectedNamespaces []mongosql.Namespace
	}{
		{
			name:               "no namespaces (implicit array datasource)",
			db:                 "test",
			sql:                "SELECT 1 + 2",
			expectedError:      false,
			expectedNamespaces: []mongosql.Namespace{},
		},
		{
			name:               "no namespaces (explicit array datasource)",
			db:                 "test",
			sql:                "SELECT * FROM [{'a': 1}] arr",
			expectedError:      false,
			expectedNamespaces: []mongosql.Namespace{},
		},
		{
			name:          "one top-level unqualified namespace",
			db:            "test",
			sql:           "SELECT * FROM foo",
			expectedError: false,
			expectedNamespaces: []mongosql.Namespace{
				{
					Database:   "test",
					Collection: "foo",
				},
			},
		},
		{
			name:          "one top-level qualified namespace (same as current db)",
			db:            "test",
			sql:           "SELECT * FROM test.foo",
			expectedError: false,
			expectedNamespaces: []mongosql.Namespace{
				{
					Database:   "test",
					Collection: "foo",
				},
			},
		},
		{
			name:          "one top-level qualified namespace (different than current db)",
			db:            "test",
			sql:           "SELECT * FROM db2.foo",
			expectedError: false,
			expectedNamespaces: []mongosql.Namespace{
				{
					Database:   "db2",
					Collection: "foo",
				},
			},
		},
		{
			name:          "multiple top-level qualified and unqualified namespaces",
			db:            "test",
			sql:           "SELECT * FROM foo, test.bar, db2.baz, xyz",
			expectedError: false,
			expectedNamespaces: []mongosql.Namespace{
				{
					Database:   "db2",
					Collection: "baz",
				},
				{
					Database:   "test",
					Collection: "bar",
				},
				{
					Database:   "test",
					Collection: "foo",
				},
				{
					Database:   "test",
					Collection: "xyz",
				},
			},
		},
		{
			name:          "namespaces nested in derived table",
			db:            "test",
			sql:           "SELECT * FROM (SELECT * FROM foo) d",
			expectedError: false,
			expectedNamespaces: []mongosql.Namespace{
				{
					Database:   "test",
					Collection: "foo",
				},
			},
		},
		{
			name:          "namespaces nested in subquery expression",
			db:            "test",
			sql:           "SELECT (SELECT bar.x IS INT FROM bar LIMIT 1) FROM foo",
			expectedError: false,
			expectedNamespaces: []mongosql.Namespace{
				{
					Database:   "test",
					Collection: "bar",
				},
				{
					Database:   "test",
					Collection: "foo",
				},
			},
		},
		{
			name:          "namespaces set query",
			db:            "test",
			sql:           "SELECT * FROM foo UNION ALL SELECT * FROM bar",
			expectedError: false,
			expectedNamespaces: []mongosql.Namespace{
				{
					Database:   "test",
					Collection: "bar",
				},
				{
					Database:   "test",
					Collection: "foo",
				},
			},
		},
		{
			name:          "relaxed schema checking",
			db:            "test",
			sql:           "SELECT * FROM foo ORDER BY _id",
			expectedError: false,
			expectedNamespaces: []mongosql.Namespace{
				{
					Database:   "test",
					Collection: "foo",
				},
			},
		},
		{
			name:               "syntactically invalid query errors",
			db:                 "test",
			sql:                "SELECT * FROM [{'a': 1}]", // no alias is syntactically invalid
			expectedError:      true,
			expectedNamespaces: nil,
		},
		{
			name:          "semantically invalid query does not error",
			db:            "test",
			sql:           "SELECT x FROM foo JOIN bar", // x is ambiguous
			expectedError: false,
			expectedNamespaces: []mongosql.Namespace{
				{
					Database:   "test",
					Collection: "bar",
				},
				{
					Database:   "test",
					Collection: "foo",
				},
			},
		},
	}

	for _, test := range tests {
		if test.skipReason != "" {
			continue
		}
		t.Run(test.name, func(t *testing.T) {
			actualNamespaces, err := mongosql.GetNamespaces(test.db, test.sql)
			if err != nil && test.expectedError {
				return
			}
			if err != nil && !test.expectedError {
				t.Fatalf("unexpected error: %v", err)
			}
			if err == nil && test.expectedError {
				t.Fatalf("expected error but none was returned")
			}

			if !cmp.Equal(test.expectedNamespaces, actualNamespaces) {
				t.Fatalf("actual namespaces (+) do not match expected namespaces (-): %v", cmp.Diff(test.expectedNamespaces, actualNamespaces))
			}
		})
	}
}
