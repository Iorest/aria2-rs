//! Pure-Rust port of aria2@1.37.0 `src/uri_split.c` plus the scalar wrapper in
//! `c-original/uri_split_wrap.c`. The state machine below is a line-for-line
//! translation of `uri_split()`; field offsets are byte offsets in the input.
//!
//! `lib.rs` owns the only `unsafe` code (the C string -> bytes conversion).

#![forbid(unsafe_code)]
// This is a line-for-line port of a C state machine: some C locals are written
// on every transition but only their presence is observed by the scalar
// wrapper. Keep those assignments so the control flow stays auditable.
#![allow(unused_assignments, unused_variables, clippy::collapsible_if)]

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    BeforeScheme,
    Scheme,
    SchemeSlash1,
    SchemeSlash2,
    BeforeMaybeUser,
    MaybeUser,
    BeforeMaybePasswd,
    MaybePasswd,
    BeforeHost,
    Host,
    BeforeIpv6Host,
    Ipv6Host,
    AfterIpv6Host,
    BeforePort,
    Port,
    Path,
    BeforeQuery,
    Query,
    BeforeFragment,
    Fragment,
}

const USR_SCHEME: u16 = 1 << 0;
const USR_HOST: u16 = 1 << 1;
const USR_PORT: u16 = 1 << 2;
const USR_PATH: u16 = 1 << 3;
const USR_QUERY: u16 = 1 << 4;
const USR_FRAGMENT: u16 = 1 << 5;
const USR_USERINFO: u16 = 1 << 6;
const USR_USER: u16 = 1 << 7;
const USR_PASSWD: u16 = 1 << 8;
const USR_BASENAME: u16 = 1 << 9;
const USF_IPV6ADDR: u8 = 1;

#[derive(Default)]
struct Split {
    field_set: u16,
    port: u16,
    flags: u8,
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn set(field_set: &mut u16, bit: u16, first: Option<usize>) {
    if first.is_some() {
        *field_set |= bit;
    }
}

/// Port of `int64_t uri_split_summary(const char* uri)`.
///
/// `uri`: the bytes of the C string, without its NUL; `None` for a null pointer
/// (the C wrapper does not accept it, so it is reported as a parse failure).
pub fn uri_split_summary(uri: Option<&[u8]>) -> i64 {
    let Some(bytes) = uri else {
        return -1;
    };
    let Some(result) = split(bytes) else {
        return -1;
    };
    let mut out = u64::from(result.field_set);
    out |= u64::from(result.port as u32) << 32;
    out |= u64::from(result.flags) << 48;
    // The wrapper returns int64_t; every packed component fits in 48 bits.
    out as i64
}

fn split(uri: &[u8]) -> Option<Split> {
    use State::*;

    let mut state = BeforeScheme;
    let mut scheme_first = None;
    let mut scheme_last = None;
    let mut host_first = None;
    let mut host_last = None;
    let mut path_first = None;
    let mut path_last = None;
    let mut query_first = None;
    let mut query_last = None;
    let mut fragment_first = None;
    let mut fragment_last = None;
    let mut user_first = None;
    let mut user_last = None;
    let mut passwd_first = None;
    let mut passwd_last = None;
    let mut last_atmark = None;
    let mut last_slash = None;
    let mut port: i32 = -1;
    let mut flags: u8 = 0;

    // The C loop is `for (; *p; ++p)`, so it stops before the NUL terminator.
    // `uri` already excludes it; `p == uri.len()` is the end-of-string value.
    for (p, &c) in uri.iter().enumerate() {
        match state {
            BeforeScheme => {
                scheme_first = Some(p);
                state = Scheme;
            }
            Scheme => {
                if c == b':' {
                    scheme_last = Some(p);
                    state = SchemeSlash1;
                }
            }
            SchemeSlash1 => {
                if c == b'/' {
                    state = SchemeSlash2;
                } else {
                    return None;
                }
            }
            SchemeSlash2 => {
                if c == b'/' {
                    state = BeforeMaybeUser;
                } else {
                    return None;
                }
            }
            BeforeMaybeUser => match c {
                b'@' | b':' | b'/' => return None,
                b'[' => state = BeforeIpv6Host,
                _ => {
                    user_first = Some(p);
                    state = MaybeUser;
                }
            },
            MaybeUser => match c {
                b'@' => last_atmark = Some(p),
                b':' => {
                    user_last = Some(p);
                    state = BeforeMaybePasswd;
                }
                b'[' => {
                    if last_atmark == p.checked_sub(1) {
                        user_last = last_atmark;
                        state = BeforeIpv6Host;
                    } else {
                        return None;
                    }
                }
                b'/' | b'?' | b'#' => {
                    if let Some(at) = last_atmark {
                        host_first = Some(at + 1);
                        host_last = Some(p);
                        user_last = last_atmark;
                    } else {
                        host_first = user_first;
                        host_last = Some(p);
                        user_first = None;
                        user_last = None;
                    }
                    match c {
                        b'/' => {
                            path_first = Some(p);
                            last_slash = Some(p);
                            state = Path;
                        }
                        b'?' => state = BeforeQuery,
                        b'#' => state = BeforeFragment,
                        _ => unreachable!(),
                    }
                }
                _ => {}
            },
            BeforeMaybePasswd => {
                passwd_first = Some(p);
                match c {
                    b'@' => {
                        passwd_last = Some(p);
                        last_atmark = Some(p);
                        state = BeforeHost;
                    }
                    b'/' => return None,
                    _ => {
                        if is_digit(c) {
                            port = i32::from(c - b'0');
                        }
                        state = MaybePasswd;
                    }
                }
            }
            MaybePasswd => match c {
                b'@' => {
                    passwd_last = Some(p);
                    last_atmark = Some(p);
                    port = -1;
                    state = BeforeHost;
                }
                b'[' => return None,
                b'/' | b'?' | b'#' => {
                    if port == -1 {
                        return None;
                    }
                    let passwd_start = passwd_first?;
                    if let Some(at) = last_atmark {
                        host_first = Some(at + 1);
                        host_last = passwd_start.checked_sub(1);
                        user_last = last_atmark;
                    } else {
                        host_first = user_first;
                        host_last = passwd_start.checked_sub(1);
                        user_first = None;
                        user_last = None;
                    }
                    passwd_first = None;
                    passwd_last = None;
                    match c {
                        b'/' => {
                            path_first = Some(p);
                            last_slash = Some(p);
                            state = Path;
                        }
                        b'?' => state = BeforeQuery,
                        b'#' => state = BeforeFragment,
                        _ => unreachable!(),
                    }
                }
                _ => {
                    if port != -1 {
                        if is_digit(c) {
                            port = port.saturating_mul(10).saturating_add(i32::from(c - b'0'));
                            if port > u16::MAX as i32 {
                                port = -1;
                            }
                        } else {
                            port = -1;
                        }
                    }
                }
            },
            BeforeHost => match c {
                b':' | b'/' => return None,
                b'[' => state = BeforeIpv6Host,
                _ => {
                    host_first = Some(p);
                    state = Host;
                }
            },
            Host => match c {
                b':' => {
                    host_last = Some(p);
                    state = BeforePort;
                }
                b'/' => {
                    host_last = Some(p);
                    path_first = Some(p);
                    last_slash = Some(p);
                    state = Path;
                }
                b'?' => {
                    host_last = Some(p);
                    state = BeforeQuery;
                }
                b'#' => {
                    host_last = Some(p);
                    state = BeforeFragment;
                }
                _ => {}
            },
            BeforeIpv6Host => {
                if c == b']' {
                    return None;
                }
                host_first = Some(p);
                state = Ipv6Host;
            }
            Ipv6Host => {
                if c == b']' {
                    flags |= USF_IPV6ADDR;
                    host_last = Some(p);
                    state = AfterIpv6Host;
                }
            }
            AfterIpv6Host => match c {
                b':' => state = BeforePort,
                b'/' => {
                    path_first = Some(p);
                    last_slash = Some(p);
                    state = Path;
                }
                b'?' => state = BeforeQuery,
                b'#' => state = BeforeFragment,
                _ => return None,
            },
            BeforePort => {
                if is_digit(c) {
                    port = i32::from(c - b'0');
                    state = Port;
                } else {
                    return None;
                }
            }
            Port => match c {
                b'/' => {
                    path_first = Some(p);
                    last_slash = Some(p);
                    state = Path;
                }
                b'?' => state = BeforeQuery,
                b'#' => state = BeforeFragment,
                _ => {
                    if is_digit(c) {
                        port = port.saturating_mul(10).saturating_add(i32::from(c - b'0'));
                        if port > u16::MAX as i32 {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
            },
            Path => match c {
                b'/' => last_slash = Some(p),
                b'?' => {
                    path_last = Some(p);
                    state = BeforeQuery;
                }
                b'#' => {
                    path_last = Some(p);
                    state = BeforeFragment;
                }
                _ => {}
            },
            BeforeQuery => {
                query_first = Some(p);
                if c == b'#' {
                    query_last = Some(p);
                    state = BeforeFragment;
                } else {
                    state = Query;
                }
            }
            Query => {
                if c == b'#' {
                    query_last = Some(p);
                    state = BeforeFragment;
                }
            }
            BeforeFragment => {
                fragment_first = Some(p);
                state = Fragment;
            }
            Fragment => {}
        }
    }

    let end = uri.len();
    match state {
        BeforeScheme | Scheme | SchemeSlash1 | SchemeSlash2 | BeforeMaybeUser
        | BeforeMaybePasswd | BeforeHost | BeforeIpv6Host | Ipv6Host | BeforePort => return None,
        MaybeUser => {
            if let Some(at) = last_atmark {
                host_first = Some(at + 1);
                host_last = Some(end);
                if host_first == host_last {
                    return None;
                }
                user_last = last_atmark;
            } else {
                host_first = user_first;
                host_last = Some(end);
                user_first = None;
                user_last = None;
            }
        }
        MaybePasswd => {
            if port == -1 {
                return None;
            }
            let passwd_start = passwd_first?;
            if let Some(at) = last_atmark {
                host_first = Some(at + 1);
                host_last = passwd_start.checked_sub(1);
                user_last = last_atmark;
            } else {
                host_first = user_first;
                host_last = passwd_start.checked_sub(1);
                user_first = None;
                user_last = None;
            }
            passwd_first = None;
            passwd_last = None;
        }
        Host => host_last = Some(end),
        Port => {
            if port == -1 {
                return None;
            }
        }
        Path => path_last = Some(end),
        BeforeQuery => {
            query_first = Some(end);
            query_last = Some(end);
        }
        Query => query_last = Some(end),
        BeforeFragment => {
            fragment_first = Some(end);
            fragment_last = Some(end);
        }
        Fragment => fragment_last = Some(end),
        AfterIpv6Host => {}
    }

    let mut field_set = 0u16;
    set(&mut field_set, USR_SCHEME, scheme_first);
    set(&mut field_set, USR_HOST, host_first);
    set(&mut field_set, USR_PATH, path_first);
    set(&mut field_set, USR_QUERY, query_first);
    set(&mut field_set, USR_FRAGMENT, fragment_first);
    set(&mut field_set, USR_USER, user_first);
    set(&mut field_set, USR_PASSWD, passwd_first);
    if user_first.is_some() {
        set(&mut field_set, USR_USERINFO, user_first);
    }
    if let (Some(slash), Some(path_end)) = (last_slash, path_last) {
        if slash + 1 != path_end {
            field_set |= USR_BASENAME;
        }
    }
    if port != -1 {
        field_set |= USR_PORT;
    }

    let port = if port == -1 { 0 } else { port as u16 };
    Some(Split {
        field_set,
        port,
        flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(s: &str) -> i64 {
        uri_split_summary(Some(s.as_bytes()))
    }

    #[test]
    fn eq_uri_rejects_non_uri_arguments() {
        // aria2@1.37.0 src/uri_split.c: the scheme must be followed by "://".
        for bad in [
            ":not-a-uri",
            "",
            "http:/x",
            "http://",
            "http://127.0.0.1:abc/x",
            "héllo wörld",
        ] {
            assert_eq!(summary(bad), -1, "{bad}");
        }
    }

    #[test]
    fn eq_uri_accepts_loopback_download_uris() {
        // aria2@1.37.0 src/uri.cc parse(): default ports are filled in by the
        // caller; uri_split itself reports only an explicitly written port.
        for good in [
            "http://127.0.0.1:18080/1mib.bin",
            "https://localhost:18443/21mib.bin",
            "ftp://127.0.0.1:1821/secret/1kib.bin",
        ] {
            assert_ne!(summary(good), -1, "{good}");
            assert_ne!(summary(good) & i64::from(USR_PORT), 0, "{good}");
            assert_ne!(summary(good) & i64::from(USR_BASENAME), 0, "{good}");
        }
    }

    #[test]
    fn eq_uri_ipv6_literal_sets_flag() {
        let result = summary("http://[::1]:18080/1kib.bin");
        assert_ne!(result, -1);
        assert_ne!(result & (i64::from(USF_IPV6ADDR) << 48), 0);
    }

    #[test]
    fn eq_uri_port_overflow_rejected_in_port_state() {
        assert_eq!(summary("http://127.0.0.1:65536/x"), -1);
        assert_ne!(summary("http://127.0.0.1:65535/x"), -1);
    }
}
