//! This is a lightly modified version of the `path-clean` crate, which diverges a bit from the
//! original semantics/behavior.
//!
//! `path-clean` is a modification of a Rust port of the the `cleanname` procedure from the Plan 9 C
//! library, and is similar to [`path.Clean`](https://golang.org/pkg/path/#Clean) from the Go
//! standard library. However, unlike both of these functions, this module will not remove `..`
//! elements which begin a path.
//!
//! It works as follows:
//!
//! 1. Reduce multiple slashes to a single slash.
//! 2. Eliminate `.` path name elements (the current directory).
//! 3. Eliminate `..` path name elements (the parent directory) and the non-`.` non-`..`, element
//!    that precedes them.
//! 4. Leave intact `..` elements that begin a path.
//!
//! If the result of this process is an empty string, return the string `"."`, representing the
//! current directory.
//!
//! It performs this transform lexically, without touching the filesystem. Therefore it doesn't do
//! any symlink resolution or absolute path resolution. For more information you can see ["Getting
//! Dot-Dot Right"](https://9p.io/sys/doc/lexnames.html).
//!
//! For convenience, the [`PathClean`] trait is exposed and comes implemented for
//! [`std::path::PathBuf`].

/*
 * Copyright (c) 2018 Dan Reeves
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use std::path::{Path, PathBuf};

/// The Clean trait implements a `clean` method. It's recommended you use the provided [`clean`]
/// function.
pub trait PathClean<T> {
    fn clean(&self) -> T;
}

/// PathClean implemented for PathBuf
impl PathClean<PathBuf> for Path {
    fn clean(&self) -> PathBuf {
        PathBuf::from(clean(self.to_str().unwrap_or("")))
    }
}

pub fn clean(path: &str) -> String {
    let out = clean_internal(path.as_bytes());
    // The code only matches/modifies ascii tokens and leaves the rest of
    // the bytes as they are, so if the input string is valid utf8 the result
    // will also be valid utf8.
    unsafe { String::from_utf8_unchecked(out) }
}

/// The core implementation. It performs the following, lexically:
/// 1. Reduce multiple slashes to a single slash.
/// 2. Eliminate `.` path name elements (the current directory).
/// 3. Eliminate `..` path name elements (the parent directory) and the non-`.` non-`..`, element that precedes them.
/// 4. Leave intact `..` elements that begin a path.
///
/// If the result of this process is an empty string, return the string `"."`, representing the current directory.
fn clean_internal(path: &[u8]) -> Vec<u8> {
    static DOT: u8 = b'.';
    static SEP: u8 = b'/';

    fn is_sep(b: u8) -> bool {
        b == b'/' || b == b'\\'
    }

    if path.is_empty() {
        return vec![DOT];
    }

    let rooted = is_sep(path[0]);
    let n = path.len();

    // Invariants:
    //  - reading from path; r is index of next byte to process.
    //  - dotdot is index in out where .. must stop, either because it is the
    //    leading slash or it is a leading ../../.. prefix.
    //
    // The go code this function is based on handles already-clean paths without
    // an allocation, but I haven't done that here because I think it
    // complicates the return signature too much.
    let mut out: Vec<u8> = Vec::with_capacity(n);
    let mut r = 0;
    let mut dotdot = 0;

    if rooted {
        out.push(SEP);
        r = 1;
        dotdot = 1;
    }

    while r < n {
        if is_sep(path[r]) || path[r] == DOT && (r + 1 == n || is_sep(path[r + 1])) {
            // empty path element || . element: skip
            r += 1;
        } else if path[r] == DOT && path[r + 1] == DOT && (r + 2 == n || is_sep(path[r + 2])) {
            // .. element: remove to last separator
            r += 2;

            if out.len() > dotdot {
                // can backtrack, truncate to last separator
                let mut w = out.len() - 1;
                while w > dotdot && !is_sep(out[w]) {
                    w -= 1;
                }

                out.truncate(w);
            } else {
                // cannot backtrack, so append .. element
                if !(out.is_empty() || (rooted && out.len() == 1)) {
                    out.push(SEP);
                }
                out.push(DOT);
                out.push(DOT);
                dotdot = out.len();
            }
        } else {
            // real path element
            // add slash if needed
            if rooted && out.len() != 1 || !rooted && !out.is_empty() {
                out.push(SEP);
            }
            while r < n && !is_sep(path[r]) {
                out.push(path[r]);
                r += 1;
            }
        }
    }

    // Turn empty string into "."
    if out.is_empty() {
        out.push(DOT);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{clean, PathClean};
    use std::path::PathBuf;

    #[test]
    fn test_empty_path_is_current_dir() {
        assert_eq!(clean(""), ".");
    }

    #[test]
    fn test_clean_paths_dont_change() {
        let tests = vec![(".", "."), ("..", ".."), ("/", "/"), ("\\", "/")];

        for test in tests {
            assert_eq!(clean(test.0), test.1);
        }
    }

    #[test]
    fn test_replace_multiple_slashes() {
        let tests = vec![
            ("/", "/"),
            ("//", "/"),
            ("/\\/", "/"),
            (".//", "."),
            ("//..", "/.."),
            ("..//", ".."),
            ("/..//", "/.."),
            ("/.//./", "/"),
            ("././/./", "."),
            ("path//to///thing", "path/to/thing"),
            ("/path//to///thing", "/path/to/thing"),
        ];

        for test in tests {
            assert_eq!(clean(test.0), test.1, "original: {}", test.0);
        }
    }

    #[test]
    fn test_eliminate_current_dir() {
        let tests = vec![
            ("./", "."),
            ("/./", "/"),
            ("./test", "test"),
            ("./test/./path", "test/path"),
            ("/test/./path/", "/test/path"),
            ("test/path/.", "test/path"),
        ];

        for test in tests {
            assert_eq!(clean(test.0), test.1, "original: {}", test.0);
        }
    }

    #[test]
    fn test_eliminate_parent_dir() {
        let tests = vec![
            ("/..", "/.."),
            ("/../test", "/../test"),
            ("test/..", "."),
            ("test/path/..", "test"),
            ("test/../path", "path"),
            ("/test/../path", "/path"),
            ("test/path/../../", "."),
            ("test/path/../../..", ".."),
            ("/test/path/../../..", "/.."),
            ("/test/path/../../../..", "/../.."),
            ("test/path/../../../..", "../.."),
            ("test/path/../../another/path", "another/path"),
            ("test/path/../../another/path/..", "another"),
            ("../test", "../test"),
            ("../test/", "../test"),
            ("../test/path", "../test/path"),
            ("../test/..", ".."),
        ];

        for test in tests {
            assert_eq!(clean(test.0), test.1, "original: {}", test.0);
        }
    }

    #[test]
    fn test_pathbuf_trait() {
        assert_eq!(
            PathBuf::from("/test/../path/").clean(),
            PathBuf::from("/path")
        );
    }
}
