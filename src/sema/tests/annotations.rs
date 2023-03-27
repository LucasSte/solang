#[cfg(test)]
use crate::sema::tests::parse;

#[test]
fn invalid_parameters() {
    let src = r#"
    contract test {

    @signer(1acc1)
    function sum(int256 a, int256 b) public returns (int256) {
        if (tx.accounts.acc1.key == address(this)) {
            return a-b;
        }
        return a+b;
    }

}
    "#;
    let ns = parse(src);

    assert!(ns.diagnostics.contains_message(
        "invalid parameter for annotation"
    ));

    let src = r#"
        contract test {

    @signer(25)
    function sum(int256 a, int256 b) public returns (int256) {
        if (tx.accounts.acc1.key == address(this)) {
            return a-b;
        }
        return a+b;
    }

}
    "#;
    let ns = parse(src);
    assert!(ns.diagnostics.contains_message("invalid parameter for annotation"));
}

#[test]
fn account_not_declared() {
    let src = r#"
    contract test {

    @signer(acc2)
    function sum(int256 a, int256 b) public returns (int256) {
        if (tx.accounts.acc1.key == address(this)) {
            return a-b;
        }
        return a+b;
    }

}
    "#;
    let ns = parse(src);
    assert!(ns.diagnostics.contains_message("account 'acc1' not declared"));
}