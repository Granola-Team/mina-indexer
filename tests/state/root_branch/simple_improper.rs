// TODO simple_improper::extension

/// Extends the root branch from a non-leaf block
#[tokio::test]
async fn extension() {
    // ----- Dangling Branches -----
    //   Before  |   After
    // ------- indices ------
    //     0     |     0
    // ----------------------
    //     0     =>    0
    //     |     =>   / \
    //     1     =>  1   2
}
