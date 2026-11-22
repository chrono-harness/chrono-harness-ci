#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn expected_post_publish_prefix(diagnostics: bool) -> (out: &'static str)
    ensures
        diagnostics ==> out@ == "${{ always() && "@,
        !diagnostics ==> out@ == "${{ always() && needs.consumer.result == 'success' && needs.frontend.result == 'success' && "@,
{
    if diagnostics { "${{ always() && " }
    else { "${{ always() && needs.consumer.result == 'success' && needs.frontend.result == 'success' && " }
}
}
