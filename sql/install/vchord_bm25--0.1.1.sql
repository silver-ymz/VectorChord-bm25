/* <begin connected objects> */
/*
This file is auto generated by pgrx.

The ordering of items is not stable, it is driven by a dependency graph.
*/
/* </end connected objects> */

/* <begin connected objects> */
-- src/lib.rs:14
-- bootstrap
CREATE TYPE bm25vector;
CREATE TYPE bm25query;
/* </end connected objects> */

/* <begin connected objects> */
-- src/index/am.rs:11
-- vchord_bm25::index::am::_bm25_amhandler
CREATE FUNCTION _bm25_amhandler(internal) RETURNS index_am_handler
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', '_bm25_amhandler_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/datatype/text_bm25vector.rs:136
-- vchord_bm25::datatype::text_bm25vector::_bm25catalog_bm25vector_in
CREATE  FUNCTION "_bm25catalog_bm25vector_in"(
	"input" cstring, /* &core::ffi::c_str::CStr */
	"_oid" oid, /* pgrx_pg_sys::submodules::oids::Oid */
	"_typmod" INT /* i32 */
) RETURNS bm25vector /* vchord_bm25::datatype::memory_bm25vector::Bm25VectorOutput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', '_bm25catalog_bm25vector_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/datatype/text_bm25vector.rs:145
-- vchord_bm25::datatype::text_bm25vector::_bm25catalog_bm25vector_out
CREATE  FUNCTION "_bm25catalog_bm25vector_out"(
	"vector" bm25vector /* vchord_bm25::datatype::memory_bm25vector::Bm25VectorInput */
) RETURNS cstring /* alloc::ffi::c_str::CString */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', '_bm25catalog_bm25vector_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/datatype/binary_bm25vector.rs:30
-- vchord_bm25::datatype::binary_bm25vector::_bm25catalog_bm25vector_recv
CREATE  FUNCTION "_bm25catalog_bm25vector_recv"(
	"internal" internal, /* pgrx::datum::internal::Internal */
	"_oid" oid, /* pgrx_pg_sys::submodules::oids::Oid */
	"_typmod" INT /* i32 */
) RETURNS bm25vector /* vchord_bm25::datatype::memory_bm25vector::Bm25VectorOutput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', '_bm25catalog_bm25vector_recv_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/datatype/binary_bm25vector.rs:12
-- vchord_bm25::datatype::binary_bm25vector::_bm25catalog_bm25vector_send
CREATE  FUNCTION "_bm25catalog_bm25vector_send"(
	"vector" bm25vector /* vchord_bm25::datatype::memory_bm25vector::Bm25VectorInput */
) RETURNS bytea /* vchord_bm25::datatype::bytea::Bytea */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', '_bm25catalog_bm25vector_send_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/datatype/cast.rs:3
-- vchord_bm25::datatype::cast::_vchord_bm25_cast_array_to_bm25vector
CREATE  FUNCTION "_vchord_bm25_cast_array_to_bm25vector"(
	"array" INT[], /* pgrx::datum::array::Array<i32> */
	"_typmod" INT, /* i32 */
	"_explicit" bool /* bool */
) RETURNS bm25vector /* vchord_bm25::datatype::memory_bm25vector::Bm25VectorOutput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', '_vchord_bm25_cast_array_to_bm25vector_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/datatype/functions.rs:11
-- vchord_bm25::datatype::functions::search_bm25query
CREATE  FUNCTION "search_bm25query"(
	"target_vector" bm25vector, /* vchord_bm25::datatype::memory_bm25vector::Bm25VectorInput */
	"query" bm25query /* pgrx::heap_tuple::PgHeapTuple<pgrx::pgbox::AllocatedByRust> */
) RETURNS real /* f32 */
STRICT STABLE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_bm25query_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/token.rs:95
-- vchord_bm25::token::unicode_tokenizer_split
CREATE  FUNCTION "unicode_tokenizer_split"(
	"text" TEXT, /* &str */
	"config" bytea /* &[u8] */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'unicode_tokenizer_split_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/token.rs:197
-- requires:
--   unicode_tokenizer_split

CREATE TABLE bm25_catalog.tokenizers (
    name TEXT NOT NULL UNIQUE PRIMARY KEY,
    config BYTEA NOT NULL
);

CREATE FUNCTION unicode_tokenizer_insert_trigger()
RETURNS TRIGGER AS $$
DECLARE
    tokenizer_name TEXT := TG_ARGV[0];
    target_column TEXT := TG_ARGV[1];
BEGIN
    EXECUTE format('
    WITH 
    config AS (
        SELECT config FROM bm25_catalog.tokenizers WHERE name = %L
    ),
    new_tokens AS (
        SELECT unnest(unicode_tokenizer_split($1.%I, config)) AS token FROM config
    ),
    to_insert AS (
        SELECT token FROM new_tokens
        WHERE NOT EXISTS (
            SELECT 1 FROM bm25_catalog.%I WHERE token = new_tokens.token
        )
    )
    INSERT INTO bm25_catalog.%I (token) SELECT token FROM to_insert ON CONFLICT (token) DO NOTHING', tokenizer_name, target_column, tokenizer_name, tokenizer_name) USING NEW;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION create_unicode_tokenizer_and_trigger(tokenizer_name TEXT, table_name TEXT, source_column TEXT, target_column TEXT)
RETURNS VOID AS $body$
BEGIN
    EXECUTE format('SELECT create_tokenizer(%L, $$
        tokenizer = ''unicode''
        table = %L
        column = %L
        $$)', tokenizer_name, table_name, source_column);
    EXECUTE format('UPDATE %I SET %I = tokenize(%I, %L)', table_name, target_column, source_column, tokenizer_name);
    EXECUTE format('CREATE TRIGGER "%s_trigger_insert" BEFORE INSERT OR UPDATE OF %I ON %I FOR EACH ROW EXECUTE FUNCTION unicode_tokenizer_set_target_column_trigger(%L, %I, %I)', tokenizer_name, source_column, table_name, tokenizer_name, source_column, target_column);
END;
$body$ LANGUAGE plpgsql;
/* </end connected objects> */

/* <begin connected objects> */
-- src/token.rs:203
-- vchord_bm25::token::create_tokenizer
-- requires:
--   tokenizer_table
CREATE  FUNCTION "create_tokenizer"(
	"tokenizer_name" TEXT, /* &str */
	"config_str" TEXT /* &str */
) RETURNS void
STRICT 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'create_tokenizer_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/token.rs:382
-- vchord_bm25::token::tokenize
-- requires:
--   tokenizer_table
CREATE  FUNCTION "tokenize"(
	"content" TEXT, /* &str */
	"tokenizer_name" TEXT /* &str */
) RETURNS bm25vector /* vchord_bm25::datatype::memory_bm25vector::Bm25VectorOutput */
STRICT STABLE PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/token.rs:226
-- vchord_bm25::token::drop_tokenizer
-- requires:
--   tokenizer_table
CREATE  FUNCTION "drop_tokenizer"(
	"tokenizer_name" TEXT /* &str */
) RETURNS void
STRICT 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'drop_tokenizer_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/token.rs:417
-- vchord_bm25::token::unicode_tokenizer_set_target_column_trigger
CREATE FUNCTION "unicode_tokenizer_set_target_column_trigger"()
	RETURNS TRIGGER
	LANGUAGE c
	AS 'MODULE_PATHNAME', 'unicode_tokenizer_set_target_column_trigger_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- src/lib.rs:15
-- finalize
CREATE TYPE bm25vector (
    INPUT = _bm25catalog_bm25vector_in,
    OUTPUT = _bm25catalog_bm25vector_out,
    RECEIVE = _bm25catalog_bm25vector_recv,
    SEND = _bm25catalog_bm25vector_send,
    STORAGE = EXTERNAL,
    INTERNALLENGTH = VARIABLE,
    ALIGNMENT = double
);

CREATE CAST (int[] AS bm25vector)
    WITH FUNCTION _vchord_bm25_cast_array_to_bm25vector(int[], integer, boolean) AS IMPLICIT;

CREATE TYPE bm25query AS (
    index_oid regclass,
    query_vector bm25vector
);

CREATE FUNCTION to_bm25query(index_oid regclass, query_str text, tokenizer_name text) RETURNS bm25query
    STABLE STRICT PARALLEL SAFE LANGUAGE sql AS $$
        SELECT index_oid, tokenize(query_str, tokenizer_name);
    $$;

CREATE ACCESS METHOD bm25 TYPE INDEX HANDLER _bm25_amhandler;
COMMENT ON ACCESS METHOD bm25 IS 'vchord bm25 index access method';

CREATE OPERATOR pg_catalog.<&> (
    PROCEDURE = search_bm25query,
    LEFTARG = bm25vector,
    RIGHTARG = bm25query
);

CREATE OPERATOR FAMILY bm25_ops USING bm25;

CREATE OPERATOR CLASS bm25_ops FOR TYPE bm25vector USING bm25 FAMILY bm25_ops AS
    OPERATOR 1 pg_catalog.<&>(bm25vector, bm25query) FOR ORDER BY float_ops;
/* </end connected objects> */

