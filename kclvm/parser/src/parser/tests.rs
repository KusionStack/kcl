use crate::lexer::parse_token_streams;
use crate::parse_file;
use crate::parser::Parser;
use crate::session::ParseSession;
use compiler_base_span::span::new_byte_pos;
use compiler_base_span::{FilePathMapping, SourceMap};
use expect_test::{expect, Expect};
use kclvm_span::create_session_globals_then;
use regex::Regex;
use std::path::PathBuf;
use std::sync::Arc;

fn check_parsing_expr(src: &str, expect: Expect) {
    let sm = SourceMap::new(FilePathMapping::empty());
    let sf = sm.new_source_file(PathBuf::from("").into(), src.to_string());
    let sess = &ParseSession::with_source_map(Arc::new(sm));

    match sf.src.as_ref() {
        Some(src_from_sf) => {
            create_session_globals_then(|| {
                let stream = parse_token_streams(sess, src_from_sf.as_str(), new_byte_pos(0));
                let mut parser = Parser::new(sess, stream);
                let expr = parser.parse_expr();
                let actual = format!("{expr:?}\n");
                expect.assert_eq(&actual)
            });
        }
        None => todo!(),
    };
}

fn check_parsing_file_ast_json(filename: &str, src: &str, expect: Expect) {
    let m = crate::parse_file(filename, Some(src.into())).unwrap();
    let actual = serde_json::ser::to_string(&m).unwrap();
    let actual = format!("{actual}\n");
    expect.assert_eq(&actual)
}

fn check_parsing_type(src: &str, expect: Expect) {
    let sm = SourceMap::new(FilePathMapping::empty());
    sm.new_source_file(PathBuf::from("").into(), src.to_string());
    let sess = &ParseSession::with_source_map(Arc::new(sm));

    create_session_globals_then(|| {
        let stream = parse_token_streams(sess, src, new_byte_pos(0));
        let mut parser = Parser::new(sess, stream);
        let typ = parser.parse_type_annotation();
        let actual = format!("{typ:?}\n");
        expect.assert_eq(&actual)
    });
}

fn check_type_str(src: &str, expect: Expect) {
    let sm = SourceMap::new(FilePathMapping::empty());
    sm.new_source_file(PathBuf::from("").into(), src.to_string());
    let sess = &ParseSession::with_source_map(Arc::new(sm));

    create_session_globals_then(|| {
        let stream = parse_token_streams(sess, src, new_byte_pos(0));
        let mut parser = Parser::new(sess, stream);
        let typ = parser.parse_type_annotation();
        let actual = typ.node.to_string();
        expect.assert_eq(&actual)
    });
}

fn check_parsing_module(filename: &str, src: &str, expect: &str) {
    let m = crate::parse_file(filename, Some(src.to_string())).expect(filename);
    let actual = format!("{}\n", serde_json::ser::to_string(&m).unwrap());
    assert_eq!(actual.trim(), expect.trim());
}

#[test]
fn smoke_test_parsing_expr() {
    check_parsing_expr(
        "1\n",
        expect![[r#"
        Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }
        "#]],
    );
    check_parsing_expr(
        "\"1\"\n",
        expect![[r#"
        Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"1\"", value: "1" }), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }
        "#]],
    );
}

#[test]
fn named_literal_expr() {
    check_parsing_expr(
        r####"Undefined"####,
        expect![[r#"
        Node { node: NameConstantLit(NameConstantLit { value: Undefined }), filename: "", line: 1, column: 0, end_line: 1, end_column: 9 }
        "#]],
    );
    check_parsing_expr(
        r####"None"####,
        expect![[r#"
        Node { node: NameConstantLit(NameConstantLit { value: None }), filename: "", line: 1, column: 0, end_line: 1, end_column: 4 }
        "#]],
    );
    (
        r####"True"####,
        expect![[r#"
        Node { node: NameConstantLit(NameConstantLit { value: True }), filename: "", line: 1, column: 1, end_line: 1, end_column: 1 }
        "#]],
    );
    check_parsing_expr(
        r####"False"####,
        expect![[r#"
        Node { node: NameConstantLit(NameConstantLit { value: False }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn nonstring_literal_expr() {
    check_parsing_expr(
        r####"1234"####,
        expect![[r#"
        Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1234) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 4 }
        "#]],
    )
}

#[test]
fn string_literal_expr_0() {
    check_parsing_expr(
        r####"'1234'"####,
        expect![[r#"
        Node { node: StringLit(StringLit { is_long_string: false, raw_value: "'1234'", value: "1234" }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
        "#]],
    )
}

#[test]
fn string_literal_expr_1() {
    check_parsing_expr(
        r####""1234""####,
        expect![[r#"
        Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"1234\"", value: "1234" }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
        "#]],
    )
}

#[test]
fn string_literal_expr_2() {
    check_parsing_expr(
        r####""1234\n""####,
        expect![[r#"
        Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"1234\\n\"", value: "1234\n" }), filename: "", line: 1, column: 0, end_line: 1, end_column: 8 }
        "#]],
    )
}

#[test]
fn number_bin_suffix_expr() {
    check_parsing_expr(
        r####"1234Ki"####,
        expect![[r#"
        Node { node: NumberLit(NumberLit { binary_suffix: Some(Ki), value: Int(1234) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
        "#]],
    )
}

#[test]
fn unary_expr() {
    check_parsing_expr(
        r####"+1"####,
        expect![[r#"
        Node { node: Unary(UnaryExpr { op: UAdd, operand: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 2 }
        "#]],
    );
}

#[test]
fn binary_expr_0() {
    check_parsing_expr(
        r####"1+2+3"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, op: Bin(Add), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }, op: Bin(Add), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn binary_expr_1() {
    check_parsing_expr(
        r####"1+2*3-4"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, op: Bin(Add), right: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }, op: Bin(Mul), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 } }), filename: "", line: 1, column: 2, end_line: 1, end_column: 5 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }, op: Bin(Sub), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(4) }), filename: "", line: 1, column: 6, end_line: 1, end_column: 7 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 7 }
        "#]],
    );
}

#[test]
fn binary_expr_2() {
    check_parsing_expr(
        r####"1+2*3/4"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, op: Bin(Add), right: Node { node: Binary(BinaryExpr { left: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }, op: Bin(Mul), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 } }), filename: "", line: 1, column: 2, end_line: 1, end_column: 5 }, op: Bin(Div), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(4) }), filename: "", line: 1, column: 6, end_line: 1, end_column: 7 } }), filename: "", line: 1, column: 2, end_line: 1, end_column: 7 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 7 }
        "#]],
    );
}

#[test]
fn binary_expr_3() {
    check_parsing_expr(
        r####"a or b"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, op: Bin(Or), right: Node { node: Identifier(Identifier { names: ["b"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 5, end_line: 1, end_column: 6 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
    "#]],
    );
}

#[test]
fn binary_expr_4() {
    check_parsing_expr(
        r####"x == a or b"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["x"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, ops: [Eq], comparators: [Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 5, end_line: 1, end_column: 6 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 11 }, op: Bin(Or), right: Node { node: Identifier(Identifier { names: ["b"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 10, end_line: 1, end_column: 11 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 11 }
        "#]],
    );
    check_parsing_expr(
        r####"22 > 11 and 111 < 222"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: Compare(Compare { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(22) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 2 }, ops: [Gt], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(11) }), filename: "", line: 1, column: 5, end_line: 1, end_column: 7 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 21 }, op: Bin(And), right: Node { node: Compare(Compare { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(111) }), filename: "", line: 1, column: 12, end_line: 1, end_column: 15 }, ops: [Lt], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(222) }), filename: "", line: 1, column: 18, end_line: 1, end_column: 21 }] }), filename: "", line: 1, column: 12, end_line: 1, end_column: 21 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 21 }
        "#]],
    );
    check_parsing_expr(
        r####"int(e.value) > 1 and i == 0"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: Compare(Compare { left: Node { node: Call(CallExpr { func: Node { node: Identifier(Identifier { names: ["int"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }, args: [Node { node: Identifier(Identifier { names: ["e", "value"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 4, end_line: 1, end_column: 11 }], keywords: [] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 12 }, ops: [Gt], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 15, end_line: 1, end_column: 16 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 27 }, op: Bin(And), right: Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["i"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 21, end_line: 1, end_column: 22 }, ops: [Eq], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 26, end_line: 1, end_column: 27 }] }), filename: "", line: 1, column: 21, end_line: 1, end_column: 27 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 27 }
        "#]],
    );
    check_parsing_expr(
        r####"key in ['key']"####,
        expect![[r#"
        Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["key"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }, ops: [In], comparators: [Node { node: List(ListExpr { elts: [Node { node: StringLit(StringLit { is_long_string: false, raw_value: "'key'", value: "key" }), filename: "", line: 1, column: 8, end_line: 1, end_column: 13 }], ctx: Load }), filename: "", line: 1, column: 7, end_line: 1, end_column: 14 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 14 }
        "#]],
    );
    check_parsing_expr(
        r####"key not in ['key']"####,
        expect![[r#"
        Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["key"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }, ops: [NotIn], comparators: [Node { node: List(ListExpr { elts: [Node { node: StringLit(StringLit { is_long_string: false, raw_value: "'key'", value: "key" }), filename: "", line: 1, column: 12, end_line: 1, end_column: 17 }], ctx: Load }), filename: "", line: 1, column: 11, end_line: 1, end_column: 18 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 18 }
        "#]],
    );

    check_parsing_expr(
        r####"1 is 1 and 11 is not 22"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: Compare(Compare { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, ops: [Is], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 5, end_line: 1, end_column: 6 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 23 }, op: Bin(And), right: Node { node: Compare(Compare { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(11) }), filename: "", line: 1, column: 11, end_line: 1, end_column: 13 }, ops: [IsNot], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(22) }), filename: "", line: 1, column: 21, end_line: 1, end_column: 23 }] }), filename: "", line: 1, column: 11, end_line: 1, end_column: 23 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 23 }
        "#]],
    );
}

#[test]
fn binary_expr_5() {
    check_parsing_expr(
        r####"1 + a and b"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, op: Bin(Add), right: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }, op: Bin(And), right: Node { node: Identifier(Identifier { names: ["b"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 10, end_line: 1, end_column: 11 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 11 }
    "#]],
    );
}

#[test]
fn binary_expr_with_paren() {
    check_parsing_expr(
        r####"1*(2+3)-4"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, op: Bin(Mul), right: Node { node: Paren(ParenExpr { expr: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 4 }, op: Bin(Add), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 5, end_line: 1, end_column: 6 } }), filename: "", line: 1, column: 3, end_line: 1, end_column: 6 } }), filename: "", line: 1, column: 2, end_line: 1, end_column: 7 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 7 }, op: Bin(Sub), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(4) }), filename: "", line: 1, column: 8, end_line: 1, end_column: 9 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 9 }
        "#]],
    );
}

#[test]
fn logic_expr_0() {
    check_parsing_expr(
        r####"0 < a < 100"####,
        expect![[r#"
        Node { node: Compare(Compare { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, ops: [Lt, Lt], comparators: [Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(100) }), filename: "", line: 1, column: 8, end_line: 1, end_column: 11 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 11 }
        "#]],
    );
    check_parsing_expr(
        r####"0 < a < 100 + a"####,
        expect![[r#"
        Node { node: Compare(Compare { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, ops: [Lt, Lt], comparators: [Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }, Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(100) }), filename: "", line: 1, column: 8, end_line: 1, end_column: 11 }, op: Bin(Add), right: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 14, end_line: 1, end_column: 15 } }), filename: "", line: 1, column: 8, end_line: 1, end_column: 15 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 15 }
        "#]],
    );
    check_parsing_expr(
        r####"100 > a > 0"####,
        expect![[r#"
        Node { node: Compare(Compare { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(100) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }, ops: [Gt, Gt], comparators: [Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 6, end_line: 1, end_column: 7 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 10, end_line: 1, end_column: 11 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 11 }
        "#]],
    );
    check_parsing_expr(
        r####"100 + a > a > 0"####,
        expect![[r#"
        Node { node: Compare(Compare { left: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(100) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }, op: Bin(Add), right: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 6, end_line: 1, end_column: 7 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 7 }, ops: [Gt, Gt], comparators: [Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 10, end_line: 1, end_column: 11 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 14, end_line: 1, end_column: 15 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 15 }
        "#]],
    );
}

#[test]
fn logic_expr_1() {
    check_parsing_expr(
        r####"a is b"####,
        expect![[r#"
        Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, ops: [Is], comparators: [Node { node: Identifier(Identifier { names: ["b"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 5, end_line: 1, end_column: 6 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
        "#]],
    );
    check_parsing_expr(
        r####"a is not True"####,
        expect![[r#"
        Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, ops: [IsNot], comparators: [Node { node: NameConstantLit(NameConstantLit { value: True }), filename: "", line: 1, column: 9, end_line: 1, end_column: 13 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 13 }
        "#]],
    );
    check_parsing_expr(
        r####"not False or a > 0 and b is True"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: Unary(UnaryExpr { op: Not, operand: Node { node: NameConstantLit(NameConstantLit { value: False }), filename: "", line: 1, column: 4, end_line: 1, end_column: 9 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 9 }, op: Bin(Or), right: Node { node: Binary(BinaryExpr { left: Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 13, end_line: 1, end_column: 14 }, ops: [Gt], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 17, end_line: 1, end_column: 18 }] }), filename: "", line: 1, column: 13, end_line: 1, end_column: 32 }, op: Bin(And), right: Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["b"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 23, end_line: 1, end_column: 24 }, ops: [Is], comparators: [Node { node: NameConstantLit(NameConstantLit { value: True }), filename: "", line: 1, column: 28, end_line: 1, end_column: 32 }] }), filename: "", line: 1, column: 23, end_line: 1, end_column: 32 } }), filename: "", line: 1, column: 13, end_line: 1, end_column: 32 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 32 }
        "#]],
    );
}

#[test]
fn if_expr() {
    check_parsing_expr(
        r####"1 if true else 2"####,
        expect![[r#"
        Node { node: If(IfExpr { body: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, cond: Node { node: Identifier(Identifier { names: ["true"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 5, end_line: 1, end_column: 9 }, orelse: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 15, end_line: 1, end_column: 16 } }), filename: "", line: 1, column: 2, end_line: 1, end_column: 16 }
        "#]],
    );
}

#[test]
fn primary_expr_0() {
    check_parsing_expr(
        r####"a.b.c"####,
        expect![[r#"
        Node { node: Identifier(Identifier { names: ["a", "b", "c"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn primary_expr_1() {
    check_parsing_expr(
        r####"'{}'.format(1)"####,
        expect![[r#"
        Node { node: Call(CallExpr { func: Node { node: Selector(SelectorExpr { value: Node { node: StringLit(StringLit { is_long_string: false, raw_value: "'{}'", value: "{}" }), filename: "", line: 1, column: 0, end_line: 1, end_column: 4 }, attr: Node { node: Identifier { names: ["format"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 5, end_line: 1, end_column: 11 }, ctx: Load, has_question: false }), filename: "", line: 1, column: 4, end_line: 1, end_column: 11 }, args: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 12, end_line: 1, end_column: 13 }], keywords: [] }), filename: "", line: 1, column: 11, end_line: 1, end_column: 14 }
        "#]],
    );
}

#[test]
fn primary_expr_2() {
    check_parsing_expr(
        r####"str(1).isdigit()"####,
        expect![[r#"
        Node { node: Call(CallExpr { func: Node { node: Selector(SelectorExpr { value: Node { node: Call(CallExpr { func: Node { node: Identifier(Identifier { names: ["str"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }, args: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }], keywords: [] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }, attr: Node { node: Identifier { names: ["isdigit"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 7, end_line: 1, end_column: 14 }, ctx: Load, has_question: false }), filename: "", line: 1, column: 6, end_line: 1, end_column: 14 }, args: [], keywords: [] }), filename: "", line: 1, column: 14, end_line: 1, end_column: 16 }
        "#]],
    );
}

#[test]
fn list_expr() {
    check_parsing_expr(
        r####"[1, 2, 3]"####,
        expect![[r#"
        Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 7, end_line: 1, end_column: 8 }], ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 9 }
        "#]],
    );

    check_parsing_expr(
        r####"[1, if True: 2, 3]"####,
        expect![[r#"
        Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }, Node { node: ListIfItem(ListIfItemExpr { if_cond: Node { node: NameConstantLit(NameConstantLit { value: True }), filename: "", line: 1, column: 7, end_line: 1, end_column: 11 }, exprs: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 13, end_line: 1, end_column: 14 }], orelse: None }), filename: "", line: 1, column: 4, end_line: 1, end_column: 14 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 16, end_line: 1, end_column: 17 }], ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 18 }
        "#]],
    );
}

#[test]
fn list_comp_expr_0() {
    check_parsing_expr(
        r####"[x ** 2 for x in [1, 2, 3]]"####,
        expect![[r#"
        Node { node: ListComp(ListComp { elt: Node { node: Binary(BinaryExpr { left: Node { node: Identifier(Identifier { names: ["x"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }, op: Bin(Pow), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 6, end_line: 1, end_column: 7 } }), filename: "", line: 1, column: 1, end_line: 1, end_column: 7 }, generators: [Node { node: CompClause { targets: [Node { node: Identifier { names: ["x"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 12, end_line: 1, end_column: 13 }], iter: Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 18, end_line: 1, end_column: 19 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 21, end_line: 1, end_column: 22 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 24, end_line: 1, end_column: 25 }], ctx: Load }), filename: "", line: 1, column: 17, end_line: 1, end_column: 26 }, ifs: [] }, filename: "", line: 1, column: 8, end_line: 1, end_column: 26 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 27 }
        "#]],
    );
}

#[test]
fn list_comp_expr_1() {
    check_parsing_expr(
        r####"[i for i in [1, 2, 3] if i > 2]"####,
        expect![[r#"
        Node { node: ListComp(ListComp { elt: Node { node: Identifier(Identifier { names: ["i"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }, generators: [Node { node: CompClause { targets: [Node { node: Identifier { names: ["i"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 7, end_line: 1, end_column: 8 }], iter: Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 13, end_line: 1, end_column: 14 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 16, end_line: 1, end_column: 17 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 19, end_line: 1, end_column: 20 }], ctx: Load }), filename: "", line: 1, column: 12, end_line: 1, end_column: 21 }, ifs: [Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["i"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 25, end_line: 1, end_column: 26 }, ops: [Gt], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 29, end_line: 1, end_column: 30 }] }), filename: "", line: 1, column: 25, end_line: 1, end_column: 30 }] }, filename: "", line: 1, column: 3, end_line: 1, end_column: 30 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 31 }
        "#]],
    );
}

#[test]
fn dict_expr() {
    check_parsing_expr(
        r####"{k0=v0, k1=v1}"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k0"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 1, end_line: 1, end_column: 3 }), value: Node { node: Identifier(Identifier { names: ["v0"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 4, end_line: 1, end_column: 6 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 1, end_line: 1, end_column: 6 }, Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k1"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 8, end_line: 1, end_column: 10 }), value: Node { node: Identifier(Identifier { names: ["v1"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 11, end_line: 1, end_column: 13 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 8, end_line: 1, end_column: 13 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 14 }
        "#]],
    );
}

#[test]
fn dict_comp_expr() {
    check_parsing_expr(
        r####"{k: v + 1 for k, v in {k1 = 1, k2 = 2}}"####,
        expect![[r#"
        Node { node: DictComp(DictComp { entry: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }), value: Node { node: Binary(BinaryExpr { left: Node { node: Identifier(Identifier { names: ["v"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }, op: Bin(Add), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 8, end_line: 1, end_column: 9 } }), filename: "", line: 1, column: 4, end_line: 1, end_column: 9 }, operation: Union, insert_index: -1 }, generators: [Node { node: CompClause { targets: [Node { node: Identifier { names: ["k"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 14, end_line: 1, end_column: 15 }, Node { node: Identifier { names: ["v"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 17, end_line: 1, end_column: 18 }], iter: Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k1"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 23, end_line: 1, end_column: 25 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 28, end_line: 1, end_column: 29 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 23, end_line: 1, end_column: 29 }, Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k2"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 31, end_line: 1, end_column: 33 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 36, end_line: 1, end_column: 37 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 31, end_line: 1, end_column: 37 }] }), filename: "", line: 1, column: 22, end_line: 1, end_column: 38 }, ifs: [] }, filename: "", line: 1, column: 10, end_line: 1, end_column: 38 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 39 }
        "#]],
    );
}

#[test]
fn subscript_expr_0() {
    check_parsing_expr(
        r####"a[0]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), lower: None, upper: None, step: None, ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 4 }
        "#]],
    );
}

#[test]
fn subscript_expr_1() {
    check_parsing_expr(
        r####"b["k"]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["b"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: Some(Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"k\"", value: "k" }), filename: "", line: 1, column: 2, end_line: 1, end_column: 5 }), lower: None, upper: None, step: None, ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 6 }
        "#]],
    );
}

#[test]
fn subscript_expr_2() {
    check_parsing_expr(
        r####"c?[1]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["c"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 4 }), lower: None, upper: None, step: None, ctx: Load, has_question: true }), filename: "", line: 1, column: 1, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn subscript_expr_3() {
    check_parsing_expr(
        r####"a[1:]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), upper: None, step: None, ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn subscript_expr_4() {
    check_parsing_expr(
        r####"a[:-1]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: None, upper: Some(Node { node: Unary(UnaryExpr { op: USub, operand: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 } }), filename: "", line: 1, column: 3, end_line: 1, end_column: 5 }), step: None, ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 6 }
        "#]],
    );
}

#[test]
fn subscript_expr_5() {
    check_parsing_expr(
        r####"a[1:len]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), upper: Some(Node { node: Identifier(Identifier { names: ["len"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 4, end_line: 1, end_column: 7 }), step: None, ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 8 }
        "#]],
    );
}

#[test]
fn subscript_expr_6() {
    check_parsing_expr(
        r####"a[0:-1]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), upper: Some(Node { node: Unary(UnaryExpr { op: USub, operand: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 5, end_line: 1, end_column: 6 } }), filename: "", line: 1, column: 4, end_line: 1, end_column: 6 }), step: None, ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 7 }
        "#]],
    );
}

#[test]
fn subscript_expr_7() {
    check_parsing_expr(
        r####"a[::]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: None, upper: None, step: None, ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn subscript_expr_8() {
    check_parsing_expr(
        r####"a[1::]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), upper: None, step: None, ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 6 }
        "#]],
    );
}

#[test]
fn subscript_expr_9() {
    check_parsing_expr(
        r####"a[:0:]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: None, upper: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 4 }), step: None, ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 6 }
        "#]],
    );
}

#[test]
fn subscript_expr_10() {
    check_parsing_expr(
        r####"a[::-1]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: None, upper: None, step: Some(Node { node: Unary(UnaryExpr { op: USub, operand: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 5, end_line: 1, end_column: 6 } }), filename: "", line: 1, column: 4, end_line: 1, end_column: 6 }), ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 7 }
        "#]],
    );
}

#[test]
fn subscript_expr_11() {
    check_parsing_expr(
        r####"a[1::2]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), upper: None, step: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 5, end_line: 1, end_column: 6 }), ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 7 }
        "#]],
    );
}

#[test]
fn subscript_expr_12() {
    check_parsing_expr(
        r####"a[:2:1]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: None, upper: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 4 }), step: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 5, end_line: 1, end_column: 6 }), ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 7 }
        "#]],
    );
}

#[test]
fn subscript_expr_13() {
    check_parsing_expr(
        r####"a[1:2:]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), upper: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }), step: None, ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 7 }
        "#]],
    );
}

#[test]
fn subscript_expr_14() {
    check_parsing_expr(
        r####"a[1:3:1]"####,
        expect![[r#"
        Node { node: Subscript(Subscript { value: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, index: None, lower: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), upper: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }), step: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 6, end_line: 1, end_column: 7 }), ctx: Load, has_question: false }), filename: "", line: 1, column: 1, end_line: 1, end_column: 8 }
        "#]],
    );
}

#[test]
fn call_expr_0() {
    check_parsing_expr(
        r####"func0()"####,
        expect![[r#"
        Node { node: Call(CallExpr { func: Node { node: Identifier(Identifier { names: ["func0"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }, args: [], keywords: [] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 7 }
        "#]],
    );
}

#[test]
fn call_expr_1() {
    check_parsing_expr(
        r####"func1(1)"####,
        expect![[r#"
        Node { node: Call(CallExpr { func: Node { node: Identifier(Identifier { names: ["func1"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }, args: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 6, end_line: 1, end_column: 7 }], keywords: [] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 8 }
        "#]],
    );
}

#[test]
fn call_expr_2() {
    check_parsing_expr(
        r####"func2(x=2)"####,
        expect![[r#"
        Node { node: Call(CallExpr { func: Node { node: Identifier(Identifier { names: ["func2"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }, args: [], keywords: [Node { node: Keyword { arg: Node { node: Identifier { names: ["x"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 6, end_line: 1, end_column: 7 }, value: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 8, end_line: 1, end_column: 9 }) }, filename: "", line: 1, column: 6, end_line: 1, end_column: 9 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 10 }
        "#]],
    );
}

#[test]
fn call_expr_3() {
    check_parsing_expr(
        r####"func3(1,x=2)"####,
        expect![[r#"
        Node { node: Call(CallExpr { func: Node { node: Identifier(Identifier { names: ["func3"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }, args: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 6, end_line: 1, end_column: 7 }], keywords: [Node { node: Keyword { arg: Node { node: Identifier { names: ["x"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 8, end_line: 1, end_column: 9 }, value: Some(Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 10, end_line: 1, end_column: 11 }) }, filename: "", line: 1, column: 8, end_line: 1, end_column: 11 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 12 }
        "#]],
    );
}

#[test]
fn quant_expr_0() {
    check_parsing_expr(
        r####"all x in collection {x > 0}"####,
        expect![[r#"
        Node { node: Quant(QuantExpr { target: Node { node: Identifier(Identifier { names: ["collection"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 9, end_line: 1, end_column: 19 }, variables: [Node { node: Identifier { names: ["x"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }], op: All, test: Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["x"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 21, end_line: 1, end_column: 22 }, ops: [Gt], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 25, end_line: 1, end_column: 26 }] }), filename: "", line: 1, column: 21, end_line: 1, end_column: 26 }, if_cond: None, ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 27 }
        "#]],
    );
}

#[test]
fn quant_expr_1() {
    check_parsing_expr(
        r####"any y in collection {y < 0}"####,
        expect![[r#"
        Node { node: Quant(QuantExpr { target: Node { node: Identifier(Identifier { names: ["collection"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 9, end_line: 1, end_column: 19 }, variables: [Node { node: Identifier { names: ["y"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }], op: Any, test: Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["y"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 21, end_line: 1, end_column: 22 }, ops: [Lt], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 25, end_line: 1, end_column: 26 }] }), filename: "", line: 1, column: 21, end_line: 1, end_column: 26 }, if_cond: None, ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 27 }
        "#]],
    );
}

#[test]
fn quant_expr_2() {
    check_parsing_expr(
        r####"map x in collection {x + 1}"####,
        expect![[r#"
        Node { node: Quant(QuantExpr { target: Node { node: Identifier(Identifier { names: ["collection"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 9, end_line: 1, end_column: 19 }, variables: [Node { node: Identifier { names: ["x"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }], op: Map, test: Node { node: Binary(BinaryExpr { left: Node { node: Identifier(Identifier { names: ["x"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 21, end_line: 1, end_column: 22 }, op: Bin(Add), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 25, end_line: 1, end_column: 26 } }), filename: "", line: 1, column: 21, end_line: 1, end_column: 26 }, if_cond: None, ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 27 }
        "#]],
    );
}

#[test]
fn quant_expr_3() {
    check_parsing_expr(
        r####"filter x in collection {x > 1}"####,
        expect![[r#"
        Node { node: Quant(QuantExpr { target: Node { node: Identifier(Identifier { names: ["collection"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 12, end_line: 1, end_column: 22 }, variables: [Node { node: Identifier { names: ["x"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 7, end_line: 1, end_column: 8 }], op: Filter, test: Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["x"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 24, end_line: 1, end_column: 25 }, ops: [Gt], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 28, end_line: 1, end_column: 29 }] }), filename: "", line: 1, column: 24, end_line: 1, end_column: 29 }, if_cond: None, ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 30 }
        "#]],
    );
}

#[test]
fn quant_expr_4() {
    check_parsing_expr(
        r####"filter x in collection {x > 1}"####,
        expect![[r#"
        Node { node: Quant(QuantExpr { target: Node { node: Identifier(Identifier { names: ["collection"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 12, end_line: 1, end_column: 22 }, variables: [Node { node: Identifier { names: ["x"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 7, end_line: 1, end_column: 8 }], op: Filter, test: Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["x"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 24, end_line: 1, end_column: 25 }, ops: [Gt], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 28, end_line: 1, end_column: 29 }] }), filename: "", line: 1, column: 24, end_line: 1, end_column: 29 }, if_cond: None, ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 30 }
        "#]],
    );
}

#[test]
fn quant_expr_5() {
    check_parsing_expr(
        r####"map i, e in [{k1 = "v1", k2 = "v2"}] { e }"####,
        expect![[r#"
        Node { node: Quant(QuantExpr { target: Node { node: List(ListExpr { elts: [Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k1"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 14, end_line: 1, end_column: 16 }), value: Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"v1\"", value: "v1" }), filename: "", line: 1, column: 19, end_line: 1, end_column: 23 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 14, end_line: 1, end_column: 23 }, Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k2"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 25, end_line: 1, end_column: 27 }), value: Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"v2\"", value: "v2" }), filename: "", line: 1, column: 30, end_line: 1, end_column: 34 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 25, end_line: 1, end_column: 34 }] }), filename: "", line: 1, column: 13, end_line: 1, end_column: 35 }], ctx: Load }), filename: "", line: 1, column: 12, end_line: 1, end_column: 36 }, variables: [Node { node: Identifier { names: ["i"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }, Node { node: Identifier { names: ["e"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 7, end_line: 1, end_column: 8 }], op: Map, test: Node { node: Identifier(Identifier { names: ["e"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 39, end_line: 1, end_column: 40 }, if_cond: None, ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 42 }
        "#]],
    );
}

#[test]
fn quant_expr_6() {
    check_parsing_expr(
        r####"map i, e in [{k1 = "v1", k2 = "v2"}] { e if i > 0 }"####,
        expect![[r#"
        Node { node: Quant(QuantExpr { target: Node { node: List(ListExpr { elts: [Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k1"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 14, end_line: 1, end_column: 16 }), value: Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"v1\"", value: "v1" }), filename: "", line: 1, column: 19, end_line: 1, end_column: 23 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 14, end_line: 1, end_column: 23 }, Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k2"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 25, end_line: 1, end_column: 27 }), value: Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"v2\"", value: "v2" }), filename: "", line: 1, column: 30, end_line: 1, end_column: 34 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 25, end_line: 1, end_column: 34 }] }), filename: "", line: 1, column: 13, end_line: 1, end_column: 35 }], ctx: Load }), filename: "", line: 1, column: 12, end_line: 1, end_column: 36 }, variables: [Node { node: Identifier { names: ["i"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }, Node { node: Identifier { names: ["e"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 7, end_line: 1, end_column: 8 }], op: Map, test: Node { node: Identifier(Identifier { names: ["e"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 39, end_line: 1, end_column: 40 }, if_cond: Some(Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["i"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 44, end_line: 1, end_column: 45 }, ops: [Gt], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(0) }), filename: "", line: 1, column: 48, end_line: 1, end_column: 49 }] }), filename: "", line: 1, column: 44, end_line: 1, end_column: 49 }), ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 51 }
        "#]],
    );
}

#[test]
fn lambda_expr_0() {
    check_parsing_expr(
        r####"lambda {}"####,
        expect![[r#"
        Node { node: Lambda(LambdaExpr { args: None, return_type_str: None, body: [], return_ty: None }), filename: "", line: 1, column: 0, end_line: 1, end_column: 9 }
        "#]],
    );
}

#[test]
fn lambda_expr_1() {
    check_parsing_expr(
        r####"lambda x {}"####,
        expect![[r#"
        Node { node: Lambda(LambdaExpr { args: Some(Node { node: Arguments { args: [Node { node: Identifier { names: ["x"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 7, end_line: 1, end_column: 8 }], defaults: [None], type_annotation_list: [None], ty_list: [None] }, filename: "", line: 1, column: 7, end_line: 1, end_column: 8 }), return_type_str: None, body: [], return_ty: None }), filename: "", line: 1, column: 0, end_line: 1, end_column: 11 }
        "#]],
    );
}

#[test]
fn lambda_expr_2() {
    check_parsing_expr(
        r####"lambda x: int -> int {x}"####,
        expect![[r#"
        Node { node: Lambda(LambdaExpr { args: Some(Node { node: Arguments { args: [Node { node: Identifier { names: ["x"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 7, end_line: 1, end_column: 8 }], defaults: [None], type_annotation_list: [Some(Node { node: "int", filename: "", line: 1, column: 10, end_line: 1, end_column: 13 })], ty_list: [Some(Node { node: Basic(Int), filename: "", line: 1, column: 10, end_line: 1, end_column: 13 })] }, filename: "", line: 1, column: 7, end_line: 1, end_column: 13 }), return_type_str: Some("int"), body: [Node { node: Expr(ExprStmt { exprs: [Node { node: Identifier(Identifier { names: ["x"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 22, end_line: 1, end_column: 23 }] }), filename: "", line: 1, column: 22, end_line: 1, end_column: 23 }], return_ty: Some(Node { node: Basic(Int), filename: "", line: 1, column: 17, end_line: 1, end_column: 20 }) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 24 }
        "#]],
    );
}

#[test]
fn lambda_expr_3() {
    check_parsing_expr(
        r####"lambda {
    if True:
        _a = 1
    else:
        _a = 2
    _a
}"####,
        expect![[r#"
        Node { node: Lambda(LambdaExpr { args: None, return_type_str: None, body: [Node { node: If(IfStmt { body: [Node { node: Assign(AssignStmt { targets: [Node { node: Identifier { names: ["_a"], pkgpath: "", ctx: Store }, filename: "", line: 3, column: 8, end_line: 3, end_column: 10 }], value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 3, column: 13, end_line: 3, end_column: 14 }, type_annotation: None, ty: None }), filename: "", line: 3, column: 8, end_line: 3, end_column: 14 }], cond: Node { node: NameConstantLit(NameConstantLit { value: True }), filename: "", line: 2, column: 7, end_line: 2, end_column: 11 }, orelse: [Node { node: Assign(AssignStmt { targets: [Node { node: Identifier { names: ["_a"], pkgpath: "", ctx: Store }, filename: "", line: 5, column: 8, end_line: 5, end_column: 10 }], value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 5, column: 13, end_line: 5, end_column: 14 }, type_annotation: None, ty: None }), filename: "", line: 5, column: 8, end_line: 5, end_column: 14 }] }), filename: "", line: 2, column: 4, end_line: 6, end_column: 4 }, Node { node: Expr(ExprStmt { exprs: [Node { node: Identifier(Identifier { names: ["_a"], pkgpath: "", ctx: Load }), filename: "", line: 6, column: 4, end_line: 6, end_column: 6 }] }), filename: "", line: 6, column: 4, end_line: 6, end_column: 6 }], return_ty: None }), filename: "", line: 1, column: 0, end_line: 7, end_column: 1 }
        "#]],
    );
}

#[test]
fn config_expr_0() {
    check_parsing_expr(
        r####"{
    "name" = {
        "name": "alice"
    },
    "gender" = "female"
}"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"name\"", value: "name" }), filename: "", line: 2, column: 4, end_line: 2, end_column: 10 }), value: Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"name\"", value: "name" }), filename: "", line: 3, column: 8, end_line: 3, end_column: 14 }), value: Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"alice\"", value: "alice" }), filename: "", line: 3, column: 16, end_line: 3, end_column: 23 }, operation: Union, insert_index: -1 }, filename: "", line: 3, column: 8, end_line: 3, end_column: 23 }] }), filename: "", line: 2, column: 13, end_line: 4, end_column: 5 }, operation: Override, insert_index: -1 }, filename: "", line: 2, column: 4, end_line: 4, end_column: 5 }, Node { node: ConfigEntry { key: Some(Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"gender\"", value: "gender" }), filename: "", line: 5, column: 4, end_line: 5, end_column: 12 }), value: Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"female\"", value: "female" }), filename: "", line: 5, column: 15, end_line: 5, end_column: 23 }, operation: Override, insert_index: -1 }, filename: "", line: 5, column: 4, end_line: 5, end_column: 23 }] }), filename: "", line: 1, column: 0, end_line: 6, end_column: 1 }
        "#]],
    );
}

#[test]
fn config_expr_1() {
    check_parsing_expr(
        r####"{
    "name" = {
        "name": "alice"
    }
    "gender" = "female",
}"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"name\"", value: "name" }), filename: "", line: 2, column: 4, end_line: 2, end_column: 10 }), value: Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"name\"", value: "name" }), filename: "", line: 3, column: 8, end_line: 3, end_column: 14 }), value: Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"alice\"", value: "alice" }), filename: "", line: 3, column: 16, end_line: 3, end_column: 23 }, operation: Union, insert_index: -1 }, filename: "", line: 3, column: 8, end_line: 3, end_column: 23 }] }), filename: "", line: 2, column: 13, end_line: 4, end_column: 5 }, operation: Override, insert_index: -1 }, filename: "", line: 2, column: 4, end_line: 4, end_column: 5 }, Node { node: ConfigEntry { key: Some(Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"gender\"", value: "gender" }), filename: "", line: 5, column: 4, end_line: 5, end_column: 12 }), value: Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"female\"", value: "female" }), filename: "", line: 5, column: 15, end_line: 5, end_column: 23 }, operation: Override, insert_index: -1 }, filename: "", line: 5, column: 4, end_line: 5, end_column: 23 }] }), filename: "", line: 1, column: 0, end_line: 6, end_column: 1 }
        "#]],
    );
}

#[test]
fn config_expr_2() {
    check_parsing_expr(
        r####"{
    "name" = {
        "name": "alice",
    }
    "gender" = "female"
}"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"name\"", value: "name" }), filename: "", line: 2, column: 4, end_line: 2, end_column: 10 }), value: Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"name\"", value: "name" }), filename: "", line: 3, column: 8, end_line: 3, end_column: 14 }), value: Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"alice\"", value: "alice" }), filename: "", line: 3, column: 16, end_line: 3, end_column: 23 }, operation: Union, insert_index: -1 }, filename: "", line: 3, column: 8, end_line: 3, end_column: 23 }] }), filename: "", line: 2, column: 13, end_line: 4, end_column: 5 }, operation: Override, insert_index: -1 }, filename: "", line: 2, column: 4, end_line: 4, end_column: 5 }, Node { node: ConfigEntry { key: Some(Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"gender\"", value: "gender" }), filename: "", line: 5, column: 4, end_line: 5, end_column: 12 }), value: Node { node: StringLit(StringLit { is_long_string: false, raw_value: "\"female\"", value: "female" }), filename: "", line: 5, column: 15, end_line: 5, end_column: 23 }, operation: Override, insert_index: -1 }, filename: "", line: 5, column: 4, end_line: 5, end_column: 23 }] }), filename: "", line: 1, column: 0, end_line: 6, end_column: 1 }
        "#]],
    );
}

#[test]
fn config_if_expr_0() {
    check_parsing_expr(
        r####"{
    if True:
        a = 1
}"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: None, value: Node { node: ConfigIfEntry(ConfigIfEntryExpr { if_cond: Node { node: NameConstantLit(NameConstantLit { value: True }), filename: "", line: 2, column: 7, end_line: 2, end_column: 11 }, items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 3, column: 8, end_line: 3, end_column: 9 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 3, column: 12, end_line: 3, end_column: 13 }, operation: Override, insert_index: -1 }, filename: "", line: 3, column: 12, end_line: 3, end_column: 13 }], orelse: None }), filename: "", line: 3, column: 8, end_line: 4, end_column: 0 }, operation: Union, insert_index: -1 }, filename: "", line: 2, column: 4, end_line: 4, end_column: 0 }] }), filename: "", line: 1, column: 0, end_line: 4, end_column: 1 }
        "#]],
    );
}

#[test]
fn config_if_expr_1() {
    check_parsing_expr(
        r####"{
    if True:
        a = 1
    else:
        a = 2
}"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: None, value: Node { node: ConfigIfEntry(ConfigIfEntryExpr { if_cond: Node { node: NameConstantLit(NameConstantLit { value: True }), filename: "", line: 2, column: 7, end_line: 2, end_column: 11 }, items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 3, column: 8, end_line: 3, end_column: 9 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 3, column: 12, end_line: 3, end_column: 13 }, operation: Override, insert_index: -1 }, filename: "", line: 3, column: 12, end_line: 3, end_column: 13 }], orelse: Some(Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 5, column: 8, end_line: 5, end_column: 9 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 5, column: 12, end_line: 5, end_column: 13 }, operation: Override, insert_index: -1 }, filename: "", line: 5, column: 12, end_line: 5, end_column: 13 }] }), filename: "", line: 4, column: 4, end_line: 6, end_column: 0 }) }), filename: "", line: 3, column: 8, end_line: 4, end_column: 4 }, operation: Union, insert_index: -1 }, filename: "", line: 2, column: 4, end_line: 6, end_column: 0 }] }), filename: "", line: 1, column: 0, end_line: 6, end_column: 1 }
        "#]],
    );
}

#[test]
fn config_if_expr_2() {
    check_parsing_expr(
        r####"{
    if True:
        a = 1
    elif x > 1:
        a = 2
    else:
        a = 3
}"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: None, value: Node { node: ConfigIfEntry(ConfigIfEntryExpr { if_cond: Node { node: NameConstantLit(NameConstantLit { value: True }), filename: "", line: 2, column: 7, end_line: 2, end_column: 11 }, items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 3, column: 8, end_line: 3, end_column: 9 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 3, column: 12, end_line: 3, end_column: 13 }, operation: Override, insert_index: -1 }, filename: "", line: 3, column: 12, end_line: 3, end_column: 13 }], orelse: Some(Node { node: ConfigIfEntry(ConfigIfEntryExpr { if_cond: Node { node: Compare(Compare { left: Node { node: Identifier(Identifier { names: ["x"], pkgpath: "", ctx: Load }), filename: "", line: 4, column: 9, end_line: 4, end_column: 10 }, ops: [Gt], comparators: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 4, column: 13, end_line: 4, end_column: 14 }] }), filename: "", line: 4, column: 9, end_line: 4, end_column: 14 }, items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 5, column: 8, end_line: 5, end_column: 9 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 5, column: 12, end_line: 5, end_column: 13 }, operation: Override, insert_index: -1 }, filename: "", line: 5, column: 12, end_line: 5, end_column: 13 }], orelse: Some(Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 7, column: 8, end_line: 7, end_column: 9 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 7, column: 12, end_line: 7, end_column: 13 }, operation: Override, insert_index: -1 }, filename: "", line: 7, column: 12, end_line: 7, end_column: 13 }] }), filename: "", line: 6, column: 4, end_line: 8, end_column: 0 }) }), filename: "", line: 4, column: 4, end_line: 6, end_column: 4 }) }), filename: "", line: 3, column: 8, end_line: 4, end_column: 4 }, operation: Union, insert_index: -1 }, filename: "", line: 2, column: 4, end_line: 8, end_column: 0 }] }), filename: "", line: 1, column: 0, end_line: 8, end_column: 1 }
        "#]],
    );
}

#[test]
fn config_if_expr_3() {
    check_parsing_expr(
        r####"{
    if True:
        if False:
            a = 1
}"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: None, value: Node { node: ConfigIfEntry(ConfigIfEntryExpr { if_cond: Node { node: NameConstantLit(NameConstantLit { value: True }), filename: "", line: 2, column: 7, end_line: 2, end_column: 11 }, items: [Node { node: ConfigEntry { key: None, value: Node { node: ConfigIfEntry(ConfigIfEntryExpr { if_cond: Node { node: NameConstantLit(NameConstantLit { value: False }), filename: "", line: 3, column: 11, end_line: 3, end_column: 16 }, items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 4, column: 12, end_line: 4, end_column: 13 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 4, column: 16, end_line: 4, end_column: 17 }, operation: Override, insert_index: -1 }, filename: "", line: 4, column: 16, end_line: 4, end_column: 17 }], orelse: None }), filename: "", line: 4, column: 12, end_line: 5, end_column: 0 }, operation: Override, insert_index: -1 }, filename: "", line: 4, column: 12, end_line: 5, end_column: 0 }], orelse: None }), filename: "", line: 3, column: 8, end_line: 5, end_column: 0 }, operation: Union, insert_index: -1 }, filename: "", line: 2, column: 4, end_line: 5, end_column: 0 }] }), filename: "", line: 1, column: 0, end_line: 5, end_column: 1 }
        "#]],
    );
}

#[test]
fn schema_expr_0() {
    check_parsing_expr(
        r####"Schema {}"####,
        expect![[r#"
        Node { node: Schema(SchemaExpr { name: Node { node: Identifier { names: ["Schema"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }, args: [], kwargs: [], config: Node { node: Config(ConfigExpr { items: [] }), filename: "", line: 1, column: 7, end_line: 1, end_column: 9 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 9 }
        "#]],
    );
}

#[test]
fn schema_expr_1() {
    check_parsing_expr(
        r####"Schema {k=v}"####,
        expect![[r#"
        Node { node: Schema(SchemaExpr { name: Node { node: Identifier { names: ["Schema"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }, args: [], kwargs: [], config: Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 8, end_line: 1, end_column: 9 }), value: Node { node: Identifier(Identifier { names: ["v"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 10, end_line: 1, end_column: 11 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 8, end_line: 1, end_column: 11 }] }), filename: "", line: 1, column: 7, end_line: 1, end_column: 12 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 12 }
        "#]],
    );
}

#[test]
fn schema_expr_2() {
    check_parsing_expr(
        r####"Schema () {k=v}"####,
        expect![[r#"
        Node { node: Schema(SchemaExpr { name: Node { node: Identifier { names: ["Schema"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }, args: [], kwargs: [], config: Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 11, end_line: 1, end_column: 12 }), value: Node { node: Identifier(Identifier { names: ["v"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 13, end_line: 1, end_column: 14 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 11, end_line: 1, end_column: 14 }] }), filename: "", line: 1, column: 10, end_line: 1, end_column: 15 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 15 }
        "#]],
    );
}

#[test]
fn schema_expr_3() {
    check_parsing_expr(
        r####"Schema (1, 2) {k=v}"####,
        expect![[r#"
        Node { node: Schema(SchemaExpr { name: Node { node: Identifier { names: ["Schema"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }, args: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 8, end_line: 1, end_column: 9 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 11, end_line: 1, end_column: 12 }], kwargs: [], config: Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 15, end_line: 1, end_column: 16 }), value: Node { node: Identifier(Identifier { names: ["v"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 17, end_line: 1, end_column: 18 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 15, end_line: 1, end_column: 18 }] }), filename: "", line: 1, column: 14, end_line: 1, end_column: 19 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 19 }
        "#]],
    );
}

#[test]
fn schema_expr_4() {
    check_parsing_expr(
        r####"Schema (1, 2) {
    k=v
}"####,
        expect![[r#"
        Node { node: Schema(SchemaExpr { name: Node { node: Identifier { names: ["Schema"], pkgpath: "", ctx: Load }, filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }, args: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 8, end_line: 1, end_column: 9 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 11, end_line: 1, end_column: 12 }], kwargs: [], config: Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["k"], pkgpath: "", ctx: Load }), filename: "", line: 2, column: 4, end_line: 2, end_column: 5 }), value: Node { node: Identifier(Identifier { names: ["v"], pkgpath: "", ctx: Load }), filename: "", line: 2, column: 6, end_line: 2, end_column: 7 }, operation: Override, insert_index: -1 }, filename: "", line: 2, column: 4, end_line: 2, end_column: 7 }] }), filename: "", line: 1, column: 14, end_line: 3, end_column: 1 } }), filename: "", line: 1, column: 0, end_line: 3, end_column: 1 }
        "#]],
    );
}

#[test]
fn line_continue() {
    check_parsing_expr(
        r####"1 + \
2
"####,
        expect![[r#"
        Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }, op: Bin(Add), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 2, column: 0, end_line: 2, end_column: 1 } }), filename: "", line: 1, column: 0, end_line: 2, end_column: 1 }
        "#]],
    );
}

#[test]
fn test_basic_type() {
    check_parsing_type(
        r####"bool"####,
        expect![[r#"
        Node { node: Basic(Bool), filename: "", line: 1, column: 0, end_line: 1, end_column: 4 }
        "#]],
    );
    check_parsing_type(
        r####"int"####,
        expect![[r#"
        Node { node: Basic(Int), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }
        "#]],
    );
    check_parsing_type(
        r####"float"####,
        expect![[r#"
        Node { node: Basic(Float), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
    check_parsing_type(
        r####"str"####,
        expect![[r#"
        Node { node: Basic(Str), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }
        "#]],
    );
}

#[test]
fn test_any_type() {
    check_parsing_type(
        r####"any"####,
        expect![[r#"
        Node { node: Any, filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }
        "#]],
    );
}

#[test]
fn test_list_type() {
    check_parsing_type(
        r####"[]"####,
        expect![[r#"
        Node { node: List(ListType { inner_type: None }), filename: "", line: 1, column: 0, end_line: 1, end_column: 2 }
        "#]],
    );
    check_parsing_type(
        r####"[int]"####,
        expect![[r#"
        Node { node: List(ListType { inner_type: Some(Node { node: Basic(Int), filename: "", line: 1, column: 1, end_line: 1, end_column: 4 }) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
    check_parsing_type(
        r####"[any]"####,
        expect![[r#"
        Node { node: List(ListType { inner_type: Some(Node { node: Any, filename: "", line: 1, column: 1, end_line: 1, end_column: 4 }) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );

    check_parsing_type(
        r####"[[]]"####,
        expect![[r#"
        Node { node: List(ListType { inner_type: Some(Node { node: List(ListType { inner_type: None }), filename: "", line: 1, column: 1, end_line: 1, end_column: 3 }) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 4 }
        "#]],
    );
    check_parsing_type(
        r####"[[str]]"####,
        expect![[r#"
        Node { node: List(ListType { inner_type: Some(Node { node: List(ListType { inner_type: Some(Node { node: Basic(Str), filename: "", line: 1, column: 2, end_line: 1, end_column: 5 }) }), filename: "", line: 1, column: 1, end_line: 1, end_column: 6 }) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 7 }
        "#]],
    );
}

#[test]
fn test_dict_type() {
    check_parsing_type(
        r####"{:}"####,
        expect![[r#"
        Node { node: Dict(DictType { key_type: None, value_type: None }), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }
        "#]],
    );
    check_parsing_type(
        r####"{str:}"####,
        expect![[r#"
        Node { node: Dict(DictType { key_type: Some(Node { node: Basic(Str), filename: "", line: 1, column: 1, end_line: 1, end_column: 4 }), value_type: None }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
        "#]],
    );
    check_parsing_type(
        r####"{:[]}"####,
        expect![[r#"
        Node { node: Dict(DictType { key_type: None, value_type: Some(Node { node: List(ListType { inner_type: None }), filename: "", line: 1, column: 2, end_line: 1, end_column: 4 }) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
    check_parsing_type(
        r####"{str:{:float}}"####,
        expect![[r#"
        Node { node: Dict(DictType { key_type: Some(Node { node: Basic(Str), filename: "", line: 1, column: 1, end_line: 1, end_column: 4 }), value_type: Some(Node { node: Dict(DictType { key_type: None, value_type: Some(Node { node: Basic(Float), filename: "", line: 1, column: 7, end_line: 1, end_column: 12 }) }), filename: "", line: 1, column: 5, end_line: 1, end_column: 13 }) }), filename: "", line: 1, column: 0, end_line: 1, end_column: 14 }
        "#]],
    );
}

#[test]
fn test_union_type() {
    check_parsing_type(
        r####"int|str"####,
        expect![[r#"
        Node { node: Union(UnionType { type_elements: [Node { node: Basic(Int), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }, Node { node: Basic(Str), filename: "", line: 1, column: 4, end_line: 1, end_column: 7 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 7 }
        "#]],
    );
    check_parsing_type(
        r####"int | str | [] | {:}"####,
        expect![[r#"
        Node { node: Union(UnionType { type_elements: [Node { node: Basic(Int), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }, Node { node: Basic(Str), filename: "", line: 1, column: 6, end_line: 1, end_column: 9 }, Node { node: List(ListType { inner_type: None }), filename: "", line: 1, column: 12, end_line: 1, end_column: 14 }, Node { node: Dict(DictType { key_type: None, value_type: None }), filename: "", line: 1, column: 17, end_line: 1, end_column: 20 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 20 }
        "#]],
    );
}

#[test]
fn test_named_type() {
    check_parsing_type(
        r####"Person"####,
        expect![[r#"
        Node { node: Named(Identifier { names: ["Person"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
        "#]],
    );
    check_parsing_type(
        r####"some.pkg.Person"####,
        expect![[r#"
        Node { node: Named(Identifier { names: ["some", "pkg", "Person"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 15 }
        "#]],
    )
}

#[test]
fn test_literal_type() {
    check_parsing_type(
        r####"True"####,
        expect![[r#"
        Node { node: Literal(Bool(true)), filename: "", line: 1, column: 0, end_line: 1, end_column: 4 }
        "#]],
    );
    check_parsing_type(
        r####" False "####,
        expect![[r#"
        Node { node: Literal(Bool(false)), filename: "", line: 1, column: 1, end_line: 1, end_column: 6 }
        "#]],
    );

    check_parsing_type(
        r####"123"####,
        expect![[r#"
        Node { node: Literal(Int(123, None)), filename: "", line: 1, column: 0, end_line: 1, end_column: 3 }
        "#]],
    );

    check_parsing_type(
        r####"123.0"####,
        expect![[r#"
        Node { node: Literal(Float(123.0)), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );

    check_parsing_type(
        r####""abc""####,
        expect![[r#"
        Node { node: Literal(Str("abc")), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
    check_parsing_type(
        r####"''"####,
        expect![[r#"
        Node { node: Literal(Str("")), filename: "", line: 1, column: 0, end_line: 1, end_column: 2 }
        "#]],
    );
}

#[test]
fn test_type_str() {
    check_type_str(r####"int"####, expect![[r#"int"#]]);
    check_type_str(r####"  int    "####, expect![[r#"int"#]]);

    check_type_str(
        r####"bool | True |  int  | str|str"####,
        expect![[r#"bool|True|int|str|str"#]],
    );
    check_type_str(
        r####"[ [{str: float}] | int]"####,
        expect![[r#"[[{str:float}]|int]"#]],
    );
}

#[test]
fn test_parse_schema_stmt() {
    check_parsing_file_ast_json(
        "hello.k",
        r####"
schema TestBool:
    []
    [str    ]: int
    [a: str]: int
    [a: ...str]: int
    [...str]: int
    a: int
    b?: str
    c: int = 0
    d?: str = ""

    [a]
    [a, b, c]
    [
        1
    ]
    [
        a
    ]
    [a for a in [1, 2, 3]]
    [
        a for a in [1, 2, 3]
    ]

    check:
        a > 1, "msg"
        "####,
        expect![[r#"
        {"filename":"hello.k","pkg":"__main__","doc":"","name":"__main__","body":[{"node":{"Schema":{"doc":"","name":{"node":"TestBool","filename":"hello.k","line":2,"column":7,"end_line":2,"end_column":15},"parent_name":null,"for_host_name":null,"is_mixin":false,"is_protocol":false,"args":null,"mixins":[],"body":[{"node":{"Expr":{"exprs":[{"node":{"List":{"elts":[],"ctx":"Load"}},"filename":"hello.k","line":3,"column":4,"end_line":3,"end_column":6}]}},"filename":"hello.k","line":3,"column":4,"end_line":3,"end_column":6},{"node":{"SchemaAttr":{"doc":"","name":{"node":"a","filename":"hello.k","line":8,"column":4,"end_line":8,"end_column":5},"type_str":{"node":"int","filename":"hello.k","line":8,"column":7,"end_line":8,"end_column":10},"op":null,"value":null,"is_optional":false,"decorators":[],"ty":{"node":{"Basic":"Int"},"filename":"hello.k","line":8,"column":7,"end_line":8,"end_column":10}}},"filename":"hello.k","line":8,"column":4,"end_line":8,"end_column":10},{"node":{"SchemaAttr":{"doc":"","name":{"node":"b","filename":"hello.k","line":9,"column":4,"end_line":9,"end_column":5},"type_str":{"node":"str","filename":"hello.k","line":9,"column":4,"end_line":9,"end_column":5},"op":null,"value":null,"is_optional":true,"decorators":[],"ty":{"node":{"Basic":"Str"},"filename":"hello.k","line":9,"column":8,"end_line":9,"end_column":11}}},"filename":"hello.k","line":9,"column":4,"end_line":10,"end_column":0},{"node":{"SchemaAttr":{"doc":"","name":{"node":"c","filename":"hello.k","line":10,"column":4,"end_line":10,"end_column":5},"type_str":{"node":"int","filename":"hello.k","line":10,"column":7,"end_line":10,"end_column":10},"op":{"Aug":"Assign"},"value":{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":0}}},"filename":"hello.k","line":10,"column":13,"end_line":10,"end_column":14},"is_optional":false,"decorators":[],"ty":{"node":{"Basic":"Int"},"filename":"hello.k","line":10,"column":7,"end_line":10,"end_column":10}}},"filename":"hello.k","line":10,"column":4,"end_line":10,"end_column":14},{"node":{"SchemaAttr":{"doc":"","name":{"node":"d","filename":"hello.k","line":11,"column":4,"end_line":11,"end_column":5},"type_str":{"node":"str","filename":"hello.k","line":11,"column":4,"end_line":11,"end_column":5},"op":{"Aug":"Assign"},"value":{"node":{"StringLit":{"is_long_string":false,"raw_value":"\"\"","value":""}},"filename":"hello.k","line":11,"column":14,"end_line":11,"end_column":16},"is_optional":true,"decorators":[],"ty":{"node":{"Basic":"Str"},"filename":"hello.k","line":11,"column":8,"end_line":11,"end_column":11}}},"filename":"hello.k","line":11,"column":4,"end_line":13,"end_column":0},{"node":{"Expr":{"exprs":[{"node":{"List":{"elts":[{"node":{"Identifier":{"names":["a"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":13,"column":5,"end_line":13,"end_column":6}],"ctx":"Load"}},"filename":"hello.k","line":13,"column":4,"end_line":13,"end_column":7}]}},"filename":"hello.k","line":13,"column":4,"end_line":13,"end_column":7},{"node":{"Expr":{"exprs":[{"node":{"List":{"elts":[{"node":{"Identifier":{"names":["a"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":14,"column":5,"end_line":14,"end_column":6},{"node":{"Identifier":{"names":["b"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":14,"column":8,"end_line":14,"end_column":9},{"node":{"Identifier":{"names":["c"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":14,"column":11,"end_line":14,"end_column":12}],"ctx":"Load"}},"filename":"hello.k","line":14,"column":4,"end_line":14,"end_column":13}]}},"filename":"hello.k","line":14,"column":4,"end_line":14,"end_column":13},{"node":{"Expr":{"exprs":[{"node":{"List":{"elts":[{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":1}}},"filename":"hello.k","line":16,"column":8,"end_line":16,"end_column":9}],"ctx":"Load"}},"filename":"hello.k","line":15,"column":4,"end_line":17,"end_column":5}]}},"filename":"hello.k","line":15,"column":4,"end_line":17,"end_column":5},{"node":{"Expr":{"exprs":[{"node":{"List":{"elts":[{"node":{"Identifier":{"names":["a"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":19,"column":8,"end_line":19,"end_column":9}],"ctx":"Load"}},"filename":"hello.k","line":18,"column":4,"end_line":20,"end_column":5}]}},"filename":"hello.k","line":18,"column":4,"end_line":20,"end_column":5},{"node":{"Expr":{"exprs":[{"node":{"ListComp":{"elt":{"node":{"Identifier":{"names":["a"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":21,"column":5,"end_line":21,"end_column":6},"generators":[{"node":{"targets":[{"node":{"names":["a"],"pkgpath":"","ctx":"Load"},"filename":"hello.k","line":21,"column":11,"end_line":21,"end_column":12}],"iter":{"node":{"List":{"elts":[{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":1}}},"filename":"hello.k","line":21,"column":17,"end_line":21,"end_column":18},{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":2}}},"filename":"hello.k","line":21,"column":20,"end_line":21,"end_column":21},{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":3}}},"filename":"hello.k","line":21,"column":23,"end_line":21,"end_column":24}],"ctx":"Load"}},"filename":"hello.k","line":21,"column":16,"end_line":21,"end_column":25},"ifs":[]},"filename":"hello.k","line":21,"column":7,"end_line":21,"end_column":25}]}},"filename":"hello.k","line":21,"column":4,"end_line":21,"end_column":26}]}},"filename":"hello.k","line":21,"column":4,"end_line":21,"end_column":26},{"node":{"Expr":{"exprs":[{"node":{"ListComp":{"elt":{"node":{"Identifier":{"names":["a"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":23,"column":8,"end_line":23,"end_column":9},"generators":[{"node":{"targets":[{"node":{"names":["a"],"pkgpath":"","ctx":"Load"},"filename":"hello.k","line":23,"column":14,"end_line":23,"end_column":15}],"iter":{"node":{"List":{"elts":[{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":1}}},"filename":"hello.k","line":23,"column":20,"end_line":23,"end_column":21},{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":2}}},"filename":"hello.k","line":23,"column":23,"end_line":23,"end_column":24},{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":3}}},"filename":"hello.k","line":23,"column":26,"end_line":23,"end_column":27}],"ctx":"Load"}},"filename":"hello.k","line":23,"column":19,"end_line":23,"end_column":28},"ifs":[]},"filename":"hello.k","line":23,"column":10,"end_line":24,"end_column":0}]}},"filename":"hello.k","line":22,"column":4,"end_line":24,"end_column":5}]}},"filename":"hello.k","line":22,"column":4,"end_line":24,"end_column":5}],"decorators":[],"checks":[{"node":{"test":{"node":{"Compare":{"left":{"node":{"Identifier":{"names":["a"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":27,"column":8,"end_line":27,"end_column":9},"ops":["Gt"],"comparators":[{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":1}}},"filename":"hello.k","line":27,"column":12,"end_line":27,"end_column":13}]}},"filename":"hello.k","line":27,"column":8,"end_line":27,"end_column":13},"if_cond":null,"msg":{"node":{"StringLit":{"is_long_string":false,"raw_value":"\"msg\"","value":"msg"}},"filename":"hello.k","line":27,"column":15,"end_line":27,"end_column":20}},"filename":"hello.k","line":27,"column":8,"end_line":27,"end_column":20}],"index_signature":{"node":{"key_name":null,"key_type":{"node":"str","filename":"hello.k","line":7,"column":8,"end_line":7,"end_column":11},"value_type":{"node":"int","filename":"hello.k","line":7,"column":14,"end_line":7,"end_column":17},"value":null,"any_other":true,"value_ty":{"node":{"Basic":"Int"},"filename":"hello.k","line":7,"column":14,"end_line":7,"end_column":17}},"filename":"hello.k","line":7,"column":4,"end_line":8,"end_column":0}}},"filename":"hello.k","line":2,"column":0,"end_line":28,"end_column":8}],"comments":[]}
        "#]],
    );
}

#[test]
fn test_parse_joined_string() {
    check_parsing_expr(
        r####"'${123+200}'"####,
        expect![[r#"
        Node { node: JoinedString(JoinedString { is_long_string: false, values: [Node { node: FormattedValue(FormattedValue { is_long_string: false, value: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(123) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 6 }, op: Bin(Add), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(200) }), filename: "", line: 1, column: 7, end_line: 1, end_column: 10 } }), filename: "", line: 1, column: 3, end_line: 1, end_column: 10 }, format_spec: None }), filename: "", line: 1, column: 1, end_line: 1, end_column: 1 }], raw_value: "'${123+200}'" }), filename: "", line: 1, column: 0, end_line: 1, end_column: 12 }
        "#]],
    );
    check_parsing_expr(
        r####"'abc${a+1}cde'"####,
        expect![[r#"
        Node { node: JoinedString(JoinedString { is_long_string: false, values: [Node { node: StringLit(StringLit { is_long_string: false, raw_value: "abc", value: "abc" }), filename: "", line: 1, column: 1, end_line: 1, end_column: 1 }, Node { node: FormattedValue(FormattedValue { is_long_string: false, value: Node { node: Binary(BinaryExpr { left: Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 6, end_line: 1, end_column: 7 }, op: Bin(Add), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 1, column: 8, end_line: 1, end_column: 9 } }), filename: "", line: 1, column: 6, end_line: 1, end_column: 9 }, format_spec: None }), filename: "", line: 1, column: 1, end_line: 1, end_column: 1 }, Node { node: StringLit(StringLit { is_long_string: false, raw_value: "cde", value: "cde" }), filename: "", line: 1, column: 1, end_line: 1, end_column: 1 }], raw_value: "'abc${a+1}cde'" }), filename: "", line: 1, column: 0, end_line: 1, end_column: 14 }
        "#]],
    );
}

#[test]
fn test_parse_assign_stmt() {
    check_parsing_file_ast_json(
        "hello.k",
        r####"a=123"####,
        expect![[r#"
        {"filename":"hello.k","pkg":"__main__","doc":"","name":"__main__","body":[{"node":{"Assign":{"targets":[{"node":{"names":["a"],"pkgpath":"","ctx":"Store"},"filename":"hello.k","line":1,"column":0,"end_line":1,"end_column":1}],"value":{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":123}}},"filename":"hello.k","line":1,"column":2,"end_line":1,"end_column":5},"type_annotation":null,"ty":null}},"filename":"hello.k","line":1,"column":0,"end_line":1,"end_column":5}],"comments":[]}
        "#]],
    );
}

#[test]
fn test_parse_if_stmt() {
    check_parsing_file_ast_json(
        "hello.k",
        r####"
a = 10
b = 12
_condition = 0
if a == 11 or b == 13: _condition = 1
elif a == 10 and b == 12: _condition = 2
condition = _condition
        "####,
        expect![[r#"
        {"filename":"hello.k","pkg":"__main__","doc":"","name":"__main__","body":[{"node":{"Assign":{"targets":[{"node":{"names":["a"],"pkgpath":"","ctx":"Store"},"filename":"hello.k","line":2,"column":0,"end_line":2,"end_column":1}],"value":{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":10}}},"filename":"hello.k","line":2,"column":4,"end_line":2,"end_column":6},"type_annotation":null,"ty":null}},"filename":"hello.k","line":2,"column":0,"end_line":2,"end_column":6},{"node":{"Assign":{"targets":[{"node":{"names":["b"],"pkgpath":"","ctx":"Store"},"filename":"hello.k","line":3,"column":0,"end_line":3,"end_column":1}],"value":{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":12}}},"filename":"hello.k","line":3,"column":4,"end_line":3,"end_column":6},"type_annotation":null,"ty":null}},"filename":"hello.k","line":3,"column":0,"end_line":3,"end_column":6},{"node":{"Assign":{"targets":[{"node":{"names":["_condition"],"pkgpath":"","ctx":"Store"},"filename":"hello.k","line":4,"column":0,"end_line":4,"end_column":10}],"value":{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":0}}},"filename":"hello.k","line":4,"column":13,"end_line":4,"end_column":14},"type_annotation":null,"ty":null}},"filename":"hello.k","line":4,"column":0,"end_line":4,"end_column":14},{"node":{"If":{"body":[{"node":{"Assign":{"targets":[{"node":{"names":["_condition"],"pkgpath":"","ctx":"Store"},"filename":"hello.k","line":5,"column":23,"end_line":5,"end_column":33}],"value":{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":1}}},"filename":"hello.k","line":5,"column":36,"end_line":5,"end_column":37},"type_annotation":null,"ty":null}},"filename":"hello.k","line":5,"column":23,"end_line":5,"end_column":37}],"cond":{"node":{"Binary":{"left":{"node":{"Compare":{"left":{"node":{"Identifier":{"names":["a"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":5,"column":3,"end_line":5,"end_column":4},"ops":["Eq"],"comparators":[{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":11}}},"filename":"hello.k","line":5,"column":8,"end_line":5,"end_column":10}]}},"filename":"hello.k","line":5,"column":3,"end_line":5,"end_column":21},"op":{"Bin":"Or"},"right":{"node":{"Compare":{"left":{"node":{"Identifier":{"names":["b"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":5,"column":14,"end_line":5,"end_column":15},"ops":["Eq"],"comparators":[{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":13}}},"filename":"hello.k","line":5,"column":19,"end_line":5,"end_column":21}]}},"filename":"hello.k","line":5,"column":14,"end_line":5,"end_column":21}}},"filename":"hello.k","line":5,"column":3,"end_line":5,"end_column":21},"orelse":[{"node":{"If":{"body":[{"node":{"Assign":{"targets":[{"node":{"names":["_condition"],"pkgpath":"","ctx":"Store"},"filename":"hello.k","line":6,"column":26,"end_line":6,"end_column":36}],"value":{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":2}}},"filename":"hello.k","line":6,"column":39,"end_line":6,"end_column":40},"type_annotation":null,"ty":null}},"filename":"hello.k","line":6,"column":26,"end_line":6,"end_column":40}],"cond":{"node":{"Binary":{"left":{"node":{"Compare":{"left":{"node":{"Identifier":{"names":["a"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":6,"column":5,"end_line":6,"end_column":6},"ops":["Eq"],"comparators":[{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":10}}},"filename":"hello.k","line":6,"column":10,"end_line":6,"end_column":12}]}},"filename":"hello.k","line":6,"column":5,"end_line":6,"end_column":24},"op":{"Bin":"And"},"right":{"node":{"Compare":{"left":{"node":{"Identifier":{"names":["b"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":6,"column":17,"end_line":6,"end_column":18},"ops":["Eq"],"comparators":[{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":12}}},"filename":"hello.k","line":6,"column":22,"end_line":6,"end_column":24}]}},"filename":"hello.k","line":6,"column":17,"end_line":6,"end_column":24}}},"filename":"hello.k","line":6,"column":5,"end_line":6,"end_column":24},"orelse":[]}},"filename":"hello.k","line":6,"column":0,"end_line":7,"end_column":0}]}},"filename":"hello.k","line":5,"column":0,"end_line":7,"end_column":0},{"node":{"Assign":{"targets":[{"node":{"names":["condition"],"pkgpath":"","ctx":"Store"},"filename":"hello.k","line":7,"column":0,"end_line":7,"end_column":9}],"value":{"node":{"Identifier":{"names":["_condition"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":7,"column":12,"end_line":7,"end_column":22},"type_annotation":null,"ty":null}},"filename":"hello.k","line":7,"column":0,"end_line":7,"end_column":22}],"comments":[]}
        "#]],
    );

    check_parsing_file_ast_json(
        "hello.k",
        r####"
data2 = {
    **{key = "value1"}
    if a == 123: if b == 456: key = "value2"
}
        "####,
        expect![[r#"
        {"filename":"hello.k","pkg":"__main__","doc":"","name":"__main__","body":[{"node":{"Assign":{"targets":[{"node":{"names":["data2"],"pkgpath":"","ctx":"Store"},"filename":"hello.k","line":2,"column":0,"end_line":2,"end_column":5}],"value":{"node":{"Config":{"items":[{"node":{"key":null,"value":{"node":{"Config":{"items":[{"node":{"key":{"node":{"Identifier":{"names":["key"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":3,"column":7,"end_line":3,"end_column":10},"value":{"node":{"StringLit":{"is_long_string":false,"raw_value":"\"value1\"","value":"value1"}},"filename":"hello.k","line":3,"column":13,"end_line":3,"end_column":21},"operation":"Override","insert_index":-1},"filename":"hello.k","line":3,"column":7,"end_line":3,"end_column":21}]}},"filename":"hello.k","line":3,"column":6,"end_line":3,"end_column":22},"operation":"Union","insert_index":-1},"filename":"hello.k","line":3,"column":4,"end_line":3,"end_column":22},{"node":{"key":null,"value":{"node":{"ConfigIfEntry":{"if_cond":{"node":{"Compare":{"left":{"node":{"Identifier":{"names":["a"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":4,"column":7,"end_line":4,"end_column":8},"ops":["Eq"],"comparators":[{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":123}}},"filename":"hello.k","line":4,"column":12,"end_line":4,"end_column":15}]}},"filename":"hello.k","line":4,"column":7,"end_line":4,"end_column":15},"items":[{"node":{"key":null,"value":{"node":{"ConfigIfEntry":{"if_cond":{"node":{"Compare":{"left":{"node":{"Identifier":{"names":["b"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":4,"column":20,"end_line":4,"end_column":21},"ops":["Eq"],"comparators":[{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":456}}},"filename":"hello.k","line":4,"column":25,"end_line":4,"end_column":28}]}},"filename":"hello.k","line":4,"column":20,"end_line":4,"end_column":28},"items":[{"node":{"key":{"node":{"Identifier":{"names":["key"],"pkgpath":"","ctx":"Load"}},"filename":"hello.k","line":4,"column":30,"end_line":4,"end_column":33},"value":{"node":{"StringLit":{"is_long_string":false,"raw_value":"\"value2\"","value":"value2"}},"filename":"hello.k","line":4,"column":36,"end_line":4,"end_column":44},"operation":"Override","insert_index":-1},"filename":"hello.k","line":4,"column":36,"end_line":4,"end_column":44}],"orelse":null}},"filename":"hello.k","line":4,"column":30,"end_line":5,"end_column":0},"operation":"Override","insert_index":-1},"filename":"hello.k","line":4,"column":30,"end_line":5,"end_column":0}],"orelse":null}},"filename":"hello.k","line":4,"column":17,"end_line":5,"end_column":0},"operation":"Union","insert_index":-1},"filename":"hello.k","line":4,"column":4,"end_line":5,"end_column":0}]}},"filename":"hello.k","line":2,"column":8,"end_line":5,"end_column":1},"type_annotation":null,"ty":null}},"filename":"hello.k","line":2,"column":0,"end_line":5,"end_column":1}],"comments":[]}
        "#]],
    );

    check_parsing_file_ast_json(
        "hello.k",
        r####"
# comment1
a = 1
# comment22
b = 2
# comment333
c = 3 # comment4444
        "####,
        expect![[r###"
        {"filename":"hello.k","pkg":"__main__","doc":"","name":"__main__","body":[{"node":{"Assign":{"targets":[{"node":{"names":["a"],"pkgpath":"","ctx":"Store"},"filename":"hello.k","line":3,"column":0,"end_line":3,"end_column":1}],"value":{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":1}}},"filename":"hello.k","line":3,"column":4,"end_line":3,"end_column":5},"type_annotation":null,"ty":null}},"filename":"hello.k","line":3,"column":0,"end_line":3,"end_column":5},{"node":{"Assign":{"targets":[{"node":{"names":["b"],"pkgpath":"","ctx":"Store"},"filename":"hello.k","line":5,"column":0,"end_line":5,"end_column":1}],"value":{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":2}}},"filename":"hello.k","line":5,"column":4,"end_line":5,"end_column":5},"type_annotation":null,"ty":null}},"filename":"hello.k","line":5,"column":0,"end_line":5,"end_column":5},{"node":{"Assign":{"targets":[{"node":{"names":["c"],"pkgpath":"","ctx":"Store"},"filename":"hello.k","line":7,"column":0,"end_line":7,"end_column":1}],"value":{"node":{"NumberLit":{"binary_suffix":null,"value":{"Int":3}}},"filename":"hello.k","line":7,"column":4,"end_line":7,"end_column":5},"type_annotation":null,"ty":null}},"filename":"hello.k","line":7,"column":0,"end_line":7,"end_column":5}],"comments":[{"node":{"text":"# comment1"},"filename":"hello.k","line":2,"column":0,"end_line":2,"end_column":10},{"node":{"text":"# comment22"},"filename":"hello.k","line":4,"column":0,"end_line":4,"end_column":11},{"node":{"text":"# comment333"},"filename":"hello.k","line":6,"column":0,"end_line":6,"end_column":12},{"node":{"text":"# comment4444"},"filename":"hello.k","line":7,"column":6,"end_line":7,"end_column":19}]}
        "###]],
    );
}

#[test]
fn test_parse_file() {
    let filenames = vec![
        "testdata/assert-01.k",
        "testdata/assert-02.k",
        "testdata/assert-03.k",
        "testdata/assert-if-0.k",
        "testdata/assert-if-1.k",
        "testdata/assert-if-2.k",
        "testdata/assign-01.k",
        "testdata/config_expr-01.k",
        "testdata/config_expr-02.k",
        "testdata/config_expr-03.k",
        "testdata/config_expr-04.k",
        "testdata/import-01.k",
        "testdata/if-01.k",
        "testdata/if-02.k",
        "testdata/if-03.k",
        "testdata/type-01.k",
        "testdata/hello_win.k",
    ];
    for filename in filenames {
        let code = std::fs::read_to_string(filename).unwrap();
        let expect = std::fs::read_to_string(filename.to_string() + ".json").unwrap();
        check_parsing_module(
            filename.trim_start_matches("testdata/"),
            code.as_str(),
            expect.as_str(),
        );
    }
}

#[test]
fn expr_with_paren1() {
    check_parsing_expr(
        r####"(2+3)"####,
        expect![[r#"
        Node { node: Paren(ParenExpr { expr: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }, op: Bin(Add), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 4 } }), filename: "", line: 1, column: 1, end_line: 1, end_column: 4 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn expr_with_paren2() {
    check_parsing_expr(
        r####"((2+3)"####,
        expect![[r#"
        Node { node: Paren(ParenExpr { expr: Node { node: Paren(ParenExpr { expr: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }, op: Bin(Add), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 } }), filename: "", line: 1, column: 2, end_line: 1, end_column: 5 } }), filename: "", line: 1, column: 1, end_line: 1, end_column: 6 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
        "#]],
    );
}

#[test]
fn expr_with_paren3() {
    check_parsing_expr(
        r####"(2+3))"####,
        expect![[r#"
        Node { node: Paren(ParenExpr { expr: Node { node: Binary(BinaryExpr { left: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }, op: Bin(Add), right: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 4 } }), filename: "", line: 1, column: 1, end_line: 1, end_column: 4 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn expr_with_bracket1() {
    check_parsing_expr(
        r####"[2,3]"####,
        expect![[r#"
        Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 4 }], ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn expr_with_bracket2() {
    check_parsing_expr(
        r####"[[2,3]"####,
        expect![[r#"
        Node { node: List(ListExpr { elts: [Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }], ctx: Load }), filename: "", line: 1, column: 1, end_line: 1, end_column: 6 }], ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
        "#]],
    );
}

#[test]
fn expr_with_bracket3() {
    check_parsing_expr(
        r####"[2,3]]"####,
        expect![[r#"
        Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 4 }], ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn expr_with_bracket4() {
    check_parsing_expr(
        r####"[2,3"####,
        expect![[r#"
        Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(3) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 4 }], ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 4 }
        "#]],
    );
}

#[test]
fn expr_with_bracket5() {
    check_parsing_expr(
        r####"["####,
        expect![[r#"
        Node { node: List(ListExpr { elts: [], ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }
        "#]],
    );
}

#[test]
fn expr_with_bracket6() {
    check_parsing_expr(
        r####"[
    1
    2,
]
        "####,
        expect![[r#"
        Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 2, column: 4, end_line: 2, end_column: 5 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 3, column: 4, end_line: 3, end_column: 5 }], ctx: Load }), filename: "", line: 1, column: 0, end_line: 4, end_column: 1 }
        "#]],
    );
}

#[test]
fn expr_with_bracket7() {
    check_parsing_expr(
        r####"[
    1,2,
]
        "####,
        expect![[r#"
        Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 2, column: 4, end_line: 2, end_column: 5 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 2, column: 6, end_line: 2, end_column: 7 }], ctx: Load }), filename: "", line: 1, column: 0, end_line: 3, end_column: 1 }
        "#]],
    );
}

#[test]
fn expr_with_bracket8() {
    check_parsing_expr(
        r####"[
    1,2,

        "####,
        expect![[r#"
        Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 2, column: 4, end_line: 2, end_column: 5 }, Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 2, column: 6, end_line: 2, end_column: 7 }], ctx: Load }), filename: "", line: 1, column: 0, end_line: 4, end_column: 8 }
        "#]],
    );
}

#[test]
fn expr_with_brace1() {
    check_parsing_expr(
        r####"{a=2}"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 4 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 1, end_line: 1, end_column: 4 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn expr_with_brace2() {
    check_parsing_expr(
        r####"{a=2}}"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 3, end_line: 1, end_column: 4 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 1, end_line: 1, end_column: 4 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 5 }
        "#]],
    );
}

#[test]
fn expr_with_delim1() {
    check_parsing_expr(
        r####"({a=2}"####,
        expect![[r#"
        Node { node: Paren(ParenExpr { expr: Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 2, end_line: 1, end_column: 5 }] }), filename: "", line: 1, column: 1, end_line: 1, end_column: 6 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
        "#]],
    );
}

#[test]
fn expr_with_delim2() {
    check_parsing_expr(
        r####"({a=(2}"####,
        expect![[r#"
        Node { node: Paren(ParenExpr { expr: Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), value: Node { node: Paren(ParenExpr { expr: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 5, end_line: 1, end_column: 6 } }), filename: "", line: 1, column: 4, end_line: 1, end_column: 7 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 2, end_line: 1, end_column: 7 }] }), filename: "", line: 1, column: 1, end_line: 1, end_column: 7 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 7 }
        "#]],
    );
}

#[test]
fn expr_with_delim3() {
    check_parsing_expr(
        r####"{a=[2]"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 1, end_line: 1, end_column: 2 }), value: Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }], ctx: Load }), filename: "", line: 1, column: 3, end_line: 1, end_column: 6 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 1, end_line: 1, end_column: 6 }] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
        "#]],
    );
}

#[test]
fn expr_with_delim4() {
    check_parsing_expr(
        r####"[{a=2}"####,
        expect![[r#"
        Node { node: List(ListExpr { elts: [Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 4, end_line: 1, end_column: 5 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 2, end_line: 1, end_column: 5 }] }), filename: "", line: 1, column: 1, end_line: 1, end_column: 6 }], ctx: Load }), filename: "", line: 1, column: 0, end_line: 1, end_column: 6 }
        "#]],
    );
}

#[test]
fn expr_with_delim5() {
    check_parsing_expr(
        r####"({a=[2}"####,
        expect![[r#"
        Node { node: Paren(ParenExpr { expr: Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 1, column: 2, end_line: 1, end_column: 3 }), value: Node { node: List(ListExpr { elts: [Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(2) }), filename: "", line: 1, column: 5, end_line: 1, end_column: 6 }], ctx: Load }), filename: "", line: 1, column: 4, end_line: 1, end_column: 7 }, operation: Override, insert_index: -1 }, filename: "", line: 1, column: 2, end_line: 1, end_column: 7 }] }), filename: "", line: 1, column: 1, end_line: 1, end_column: 7 } }), filename: "", line: 1, column: 0, end_line: 1, end_column: 7 }
        "#]],
    );
}

#[test]
fn expr_with_delim6() {
    check_parsing_expr(
        r####"{"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [] }), filename: "", line: 1, column: 0, end_line: 1, end_column: 1 }
        "#]],
    );
}

#[test]
fn expr_with_delim7() {
    check_parsing_expr(
        r####"{
    a = 1
}"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 2, column: 4, end_line: 2, end_column: 5 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 2, column: 8, end_line: 2, end_column: 9 }, operation: Override, insert_index: -1 }, filename: "", line: 2, column: 4, end_line: 2, end_column: 9 }] }), filename: "", line: 1, column: 0, end_line: 3, end_column: 1 }
        "#]],
    );
}

#[test]
fn expr_with_delim8() {
    check_parsing_expr(
        r####"{
    a = 1
"####,
        expect![[r#"
        Node { node: Config(ConfigExpr { items: [Node { node: ConfigEntry { key: Some(Node { node: Identifier(Identifier { names: ["a"], pkgpath: "", ctx: Load }), filename: "", line: 2, column: 4, end_line: 2, end_column: 5 }), value: Node { node: NumberLit(NumberLit { binary_suffix: None, value: Int(1) }), filename: "", line: 2, column: 8, end_line: 2, end_column: 9 }, operation: Override, insert_index: -1 }, filename: "", line: 2, column: 4, end_line: 2, end_column: 9 }] }), filename: "", line: 1, column: 0, end_line: 2, end_column: 10 }
        "#]],
    );
}

#[test]
fn test_parse_file_not_found() {
    match parse_file("The file path is invalid", None) {
        Ok(_) => {
            panic!("unreachable")
        }
        Err(err_msg) => {
            assert!(
                Regex::new(r"^Failed to load KCL file 'The file path is invalid'. Because.*")
                    .unwrap()
                    .is_match(&err_msg)
            );
        }
    }
}
