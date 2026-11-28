//! 原生工件下载的状态码解析与有限重试判断；实际传输仍由 gather 负责。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

use vstd::contrib::exec_spec::ExecSpecEq;

#[cfg(verus_keep_ghost)]
#[path = "observed.rs"]
mod observed;
#[cfg(verus_keep_ghost)]
#[path = "requirements.rs"]
mod requirements;
#[cfg(verus_keep_ghost)]
pub use self::{
    observed::capture_error,
    requirements::{Reply, query_reply},
};
#[cfg(not(verus_keep_ghost))]
pub use crate::{
    observed::capture_error,
    requirements::{Reply, query_reply},
};

#[cfg(verus_keep_ghost)]
#[path = "gather.rs"]
mod gather;
#[cfg(verus_keep_ghost)]
pub use self::gather::{
    AcquisitionBudget, LatestAttempt, available_time, execution_identity, latest_attempt,
    original_attempt, original_field, transport_limits,
};
#[cfg(not(verus_keep_ghost))]
pub use crate::gather::{
    AcquisitionBudget, LatestAttempt, available_time, execution_identity, latest_attempt,
    original_attempt, original_field, transport_limits,
};

verus! {

/// A page borrows the provider rows. The native adapter exposes only their names.
pub trait ArtifactPage {
    spec fn names(&self) -> Seq<Option<Seq<char>>>;

    fn len(&self) -> (n: usize)
        ensures n == self.names().len();
    fn name(&self, i: usize) -> (name: Option<&str>)
        requires i < self.names().len(),
        ensures match name {
            Some(value) => self.names()[i as int] == Some(value@),
            None => self.names()[i as int] is None,
        };
}

pub open spec fn present<P: ArtifactPage>(pages: Seq<P>, name: Seq<char>, p: int, r: int) -> bool {
    0 <= p < pages.len() && 0 <= r < pages[p].names().len() && pages[p].names()[r] == Some(name)
}

pub open spec fn unique<P: ArtifactPage>(pages: Seq<P>, name: Seq<char>, p: int, r: int) -> bool {
    present(pages,name,p,r) && forall|a:int,b:int| #[trigger] present(pages,name,a,b) ==> a == p && b == r
}

/// Search in provider order without copying a row or allocating one entry per artifact.
pub fn unique_artifact<P: ArtifactPage>(pages: &[P], name: &str) -> (selected: Option<(usize,usize)>)
    ensures
        match selected {
            Some((p,r)) => unique(pages@,name@,p as int,r as int),
            None => forall|p:int,r:int| !(#[trigger] unique(pages@,name@,p,r)),
        },
{
    let mut found: Option<(usize,usize)> = None;
    let mut p = 0;
    while p < pages.len()
        invariant p <= pages.len(),
            forall|a:int,b:int| a < p && #[trigger] present(pages@,name@,a,b) ==>
                (found is Some && found.unwrap().0 as int == a && found.unwrap().1 as int == b),
            match found {
                Some((a,b)) => a < p && present(pages@,name@,a as int,b as int),
                None => true,
            },
        decreases pages.len()-p,
    {
        let page = &pages[p];
        let len = page.len();
        let mut r = 0;
        while r < len
            invariant p < pages.len(), page == &pages@[p as int], len == page.names().len(), r <= len,
                forall|a:int,b:int| (a < p || a == p && b < r)
                    && #[trigger] present(pages@,name@,a,b) ==> (found is Some && found.unwrap().0 as int == a && found.unwrap().1 as int == b),
                match found {
                    Some((a,b)) => (a < p || a == p && b < r) && present(pages@,name@,a as int,b as int),
                    None => true,
                },
            decreases len-r,
        {
            if let Some(value) = page.name(r) {
                if <&str>::exec_eq(value,name) {
                    if let Some((a,b)) = found {
                        proof {
                            assert(present(pages@,name@,p as int,r as int));
                            assert forall|x:int,y:int| !(#[trigger] unique(pages@,name@,x,y)) by {
                                if unique(pages@,name@,x,y) {
                                    assert(present(pages@,name@,a as int,b as int));
                                    assert(present(pages@,name@,p as int,r as int));
                                }
                            }
                        }
                        return None;
                    }
                    found = Some((p,r));
                }
            }
            r += 1;
        }
        p += 1;
    }
    proof {
        match found {
            Some((page,row)) => {
                assert forall|a:int,b:int| #[trigger] present(pages@,name@,a,b)
                    implies a == page && b == row by { }
                assert(unique(pages@,name@,page as int,row as int));
            }
            None => { assert forall|a:int,b:int| !(#[trigger] unique(pages@,name@,a,b)) by {
                if unique(pages@,name@,a,b) { assert(present(pages@,name@,a,b)); }
            } }
        }
    }
    found
}

#[derive(PartialEq, Eq, Debug)]
pub enum ArtifactError { Identity, MissingId, Endpoint }

/// The adapter constructs the canonical GitHub URL using the observed numeric ID.
pub fn admit_artifact(expired: Option<bool>, observed_run: Option<u64>, run: u64,
    observed_head: Option<&str>, head: &str, id: Option<u64>, observed_url: Option<&str>, url: &str)
    -> (result: Result<(),ArtifactError>)
    ensures ({
        let same_head = match observed_head { Some(text) => text@ == head@, None => false };
        let same_url = match observed_url { Some(text) => text@ == url@, None => false };
        if expired != Some(false) || observed_run != Some(run) || !same_head { result is Err && result.unwrap_err() is Identity }
        else if id is None || id == Some(0u64) { result is Err && result.unwrap_err() is MissingId }
        else if !same_url { result is Err && result.unwrap_err() is Endpoint }
        else { result is Ok }
    }),
{
    if expired != Some(false) || observed_run != Some(run)
        || !matches!(observed_head, Some(value) if <&str>::exec_eq(value,head)) { return Err(ArtifactError::Identity); }
    if id.is_none() || id == Some(0u64) { return Err(ArtifactError::MissingId); }
    if !matches!(observed_url, Some(value) if <&str>::exec_eq(value,url)) { return Err(ArtifactError::Endpoint); }
    Ok(())
}

pub open spec fn token_at(bytes: Seq<u8>, i: int) -> bool {
    0 <= i && i + 8 < bytes.len()
        && bytes[i] == 72 && bytes[i + 1] == 84 && bytes[i + 2] == 84
        && bytes[i + 3] == 80 && bytes[i + 4] == 32 && bytes[i + 8] == 58
        && 48 <= bytes[i + 5] <= 57 && 48 <= bytes[i + 6] <= 57
        && 48 <= bytes[i + 7] <= 57
}

pub open spec fn code_at(bytes: Seq<u8>, i: int) -> int {
    100 * (bytes[i + 5] as int - 48)
        + 10 * (bytes[i + 6] as int - 48) + bytes[i + 7] as int - 48
}

pub open spec fn prefix(bytes: Seq<u8>, end: int, status: Option<u16>) -> bool {
    match status {
        None => forall|i: int| 0 <= i < end ==> !token_at(bytes, i),
        Some(code) =>
            (exists|i: int| 0 <= i < end && token_at(bytes, i) && code_at(bytes, i) == code)
            && (forall|i: int| 0 <= i < end && token_at(bytes, i) ==> code_at(bytes, i) == code),
    }
}

pub open spec fn parsed(bytes: Seq<u8>, status: Option<u16>) -> bool {
    match status {
        Some(_) => prefix(bytes, bytes.len() as int, status),
        None => prefix(bytes, bytes.len() as int, None)
            || exists|i: int, j: int| token_at(bytes, i) && token_at(bytes, j)
                && code_at(bytes, i) != code_at(bytes, j),
    }
}

fn token(bytes: &[u8], i: usize) -> (status: Option<u16>)
    requires i + 8 < bytes.len(),
    ensures match status {
        Some(code) => token_at(bytes@, i as int) && code_at(bytes@, i as int) == code,
        None => !token_at(bytes@, i as int),
    },
{
    if bytes[i] != 72 || bytes[i + 1] != 84 || bytes[i + 2] != 84
        || bytes[i + 3] != 80 || bytes[i + 4] != 32 || bytes[i + 8] != 58
        || bytes[i + 5] < 48 || bytes[i + 5] > 57
        || bytes[i + 6] < 48 || bytes[i + 6] > 57
        || bytes[i + 7] < 48 || bytes[i + 7] > 57 { return None; }
    Some(100 * (bytes[i + 5] - 48) as u16
        + 10 * (bytes[i + 6] - 48) as u16 + (bytes[i + 7] - 48) as u16)
}

/// 只接受完整的 HTTP nnn:；重复相同状态允许，任意矛盾状态拒绝。
pub fn http_status(bytes: &[u8]) -> (status: Option<u16>)
    ensures parsed(bytes@, status),
{
    if bytes.len() < 9 { return None; }
    let end = bytes.len() - 8;
    let mut i = 0;
    let mut status: Option<u16> = None;
    while i < end
        invariant bytes.len() >= 9, end == bytes.len() - 8,
            0 <= i <= end, prefix(bytes@, i as int, status),
        decreases end - i,
    {
        if let Some(current) = token(bytes, i) {
            if let Some(prior) = status {
                if prior != current {
                    proof {
                        let previous = choose|j: int| 0 <= j < i
                            && token_at(bytes@, j) && code_at(bytes@, j) == prior;
                        assert(token_at(bytes@, previous) && token_at(bytes@, i as int)
                            && code_at(bytes@, previous) != code_at(bytes@, i as int));
                    }
                    return None;
                }
            }
            status = Some(current);
        }
        i += 1;
    }
    status
}

/// 所有外部观察由调用方提供；失败、次数、流容量、状态白名单和期限缺一不可。
pub fn retry(
    exit_ok: bool, process_failed: bool, attempts_remaining: bool,
    status_allowed: bool, stdout_remaining: usize, stderr_remaining: usize,
    delay_fits: bool,
) -> (allowed: bool)
    ensures allowed == (!exit_ok && !process_failed && attempts_remaining
        && status_allowed && stdout_remaining > 0 && stderr_remaining > 0 && delay_fits),
{
    !exit_ok && !process_failed && attempts_remaining && status_allowed
        && stdout_remaining > 0 && stderr_remaining > 0 && delay_fits
}

}
