use crate::sema::tests::parse;
use crate::Target;

#[cfg(test)]

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
    let ns = parse(src, Target::Solana);

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
    let ns = parse(src, Target::Solana);
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
    let ns = parse(src, Target::Solana);
    assert!(ns.diagnostics.contains_message("account 'acc1' not declared"));
}

#[test]
fn all_accounts_ok() {
    let src = r#"
    contract test {

    @signer(acc2)
    function sum(int256 a, int256 b) public view returns (int256) {
        if (tx.accounts.acc2.key == address(this)) {
            return a-b;
        }
        return a+b;
    }

    @mutable(acc1)
    function test_account(address ss) public view returns (bool) {
        if(ss == tx.accounts.acc1.key) {
            return true;
        }

        return false;
    }

    @reader(acc4)
    @mutableSigner(acc3)
    function test_mutable_signer(address ss) public view returns (bool) {
        if(ss == tx.accounts.acc3.key && tx.accounts.acc4.is_signer) {
            return true;
        }
        return false;
    }

}
    "#;
    let ns = parse(src, Target::Solana);
    assert_eq!(ns.diagnostics.len(), 1);
    assert!(ns.diagnostics.contains_message("found contract 'test'"));
}

// TODO:
// 2. Write preamble and runtime tests for it
// 3. Write integrations tests that verify if the accounts are properly deserialized