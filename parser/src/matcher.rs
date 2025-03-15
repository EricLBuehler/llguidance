use anyhow::{anyhow, ensure, Result};
use toktrie::{SimpleVob, TokEnv, TokenId};

use crate::{panic_utils, TokenParser};

struct MatcherInner {
    parser: TokenParser,
}

#[allow(clippy::large_enum_variant)]
enum ConstraintOrError {
    Matcher(MatcherInner),
    Error(String),
}

pub struct Matcher(ConstraintOrError);

impl Matcher {
    pub fn new(parser: Result<TokenParser>) -> Self {
        match parser {
            Ok(parser) => {
                let caps = &parser.inference_caps;
                if caps.backtrack {
                    Self::new(Err(anyhow!("backtracking not supported")))
                } else {
                    // rest of caps is ignored
                    Matcher(ConstraintOrError::Matcher(MatcherInner { parser }))
                }
            }
            Err(e) => Matcher(ConstraintOrError::Error(e.to_string())),
        }
    }

    fn with_inner<T>(&mut self, f: impl FnOnce(&mut MatcherInner) -> Result<T>) -> Result<T> {
        match &mut self.0 {
            ConstraintOrError::Matcher(ref mut inner) => {
                // We catch any panics here and transform them into regular errors.
                // They shouldn't happen, but if they do, we don't want to crash the whole program.
                let r = panic_utils::catch_unwind(std::panic::AssertUnwindSafe(|| f(inner)));
                match r {
                    Ok(r) => Ok(r),
                    Err(e) => {
                        self.0 = ConstraintOrError::Error(e.to_string());
                        Err(e)
                    }
                }
            }
            ConstraintOrError::Error(e) => Err(anyhow!("{}", e)),
        }
    }

    pub fn consume_tokens(&mut self, tokens: &[TokenId]) -> Result<()> {
        self.with_inner(|inner| {
            for &t in tokens {
                let bt = inner.parser.consume_token(t)?;
                ensure!(bt == 0, "unexpected backtracking");
            }
            Ok(())
        })
    }

    pub fn rollback(&mut self, num_tokens: usize) -> Result<()> {
        self.with_inner(|inner| inner.parser.rollback(num_tokens))
    }

    /// Compute which tokens can be consumed in the current state.
    pub fn compute_mask(&mut self) -> Result<SimpleVob> {
        self.with_inner(|inner| inner.parser.compute_mask())
    }

    /// Check whether the current parser state forces the sequence to stop.
    /// If so, puts the parser in stop state and returns true.
    /// Otherwise, returns false.
    /// This generally should be called after consume_token().
    pub fn check_stop(&mut self) -> Result<bool> {
        self.with_inner(|inner| inner.parser.check_stop())
    }

    /// Can the grammar be finished in the current state?
    /// In other words, would the current token mask allow EOS token?
    pub fn is_accepting(&mut self) -> Result<bool> {
        self.with_inner(|inner| Ok(inner.parser.is_accepting()))
    }

    /// This will always return [] for non-canonical tokenizers.
    pub fn compute_ff_tokens(&mut self) -> Vec<TokenId> {
        self.with_inner(|inner| Ok(inner.parser.compute_ff_tokens()))
            .unwrap_or_else(|_| vec![])
    }

    /// Return any bytes that are forced by the current parser state.
    /// This also works for non-canonical tokenizers.
    pub fn compute_ff_bytes(&mut self) -> Vec<u8> {
        self.with_inner(|inner| Ok(inner.parser.force_bytes()))
            .unwrap_or_else(|_| vec![])
    }

    pub fn try_consume_tokens(&mut self, tokens: &[TokenId]) -> Result<usize> {
        self.with_inner(|inner| {
            for (idx, &t) in tokens.iter().enumerate() {
                if !inner.parser.validate_token(t)? {
                    return Ok(idx);
                }
                let bt = inner.parser.consume_token(t)?;
                ensure!(bt == 0, "unexpected backtracking");
            }
            Ok(tokens.len())
        })
    }

    pub fn validate_tokens(&mut self, tokens: &[TokenId]) -> Result<usize> {
        self.with_inner(|inner| inner.parser.validate_tokens_raw(tokens))
    }

    pub fn is_error(&self) -> bool {
        matches!(self.0, ConstraintOrError::Error(_))
    }

    pub fn get_error(&self) -> Option<String> {
        match &self.0 {
            ConstraintOrError::Matcher(_) => None,
            ConstraintOrError::Error(e) => Some(e.clone()),
        }
    }

    pub fn tok_env(&self) -> Result<TokEnv> {
        match &self.0 {
            ConstraintOrError::Matcher(inner) => Ok(inner.parser.token_env.clone()),
            ConstraintOrError::Error(e) => Err(anyhow!("{}", e)),
        }
    }
}
