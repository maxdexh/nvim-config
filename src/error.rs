use std::{
    backtrace::Backtrace,
    error,
    fmt::{self, Debug, Display, Write},
    panic::Location,
    sync::Arc,
};

pub struct Error(Arc<ErrorInner>);
#[derive(Clone)]
struct ErrorInner {
    chain: ErrorChain,
    backtrace: Arc<Backtrace>,
}
impl Error {
    #[cold]
    #[track_caller]
    pub fn context<C>(mut self, ctx: C) -> Self
    where
        C: Display + Send + Sync + 'static,
    {
        let inner = Arc::make_mut(&mut self.0);
        inner.chain.context(Box::new(ctx), Some(Location::caller()));
        self
    }

    #[cold]
    #[track_caller]
    pub fn msg(msg: impl std::fmt::Display) -> Self {
        #[derive(Debug)]
        struct StrError(String);
        impl std::error::Error for StrError {}
        impl Display for StrError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                Display::fmt(&self.0, f)
            }
        }
        StrError(msg.to_string()).into()
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&*self.0, f)
    }
}
impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&*self.0, f)
    }
}

type ErrorMaybeBt = Result<Error, ErrorChain>;
macro_rules! supply_backtrace {
    ($err:expr) => {
        match $err {
            Ok(err) => err,
            Err(chain) => Error(Arc::new(ErrorInner {
                chain,
                backtrace: Arc::new(Backtrace::force_capture()),
            })),
        }
    };
}

#[cold]
fn from_arc_dyn(mut inner: Arc<dyn error::Error + Send + Sync>) -> ErrorMaybeBt {
    inner = match downcast_arc_error::<Arc<mlua::Error>>(inner) {
        Ok(error) => return from_arc_mlua(Arc::unwrap_or_clone(error)),
        Err(error) => error,
    };
    match downcast_arc_error::<mlua::Error>(inner) {
        Ok(error) => from_arc_mlua(error),
        Err(error) => from_arc_dyn_ignore_mlua(error),
    }
}
#[cold]
fn from_arc_dyn_ignore_mlua(inner: Arc<dyn error::Error + Send + Sync>) -> ErrorMaybeBt {
    match downcast_arc_error(inner) {
        Ok(inner) => Ok(Error(inner)),
        Err(error) => Err(ErrorChain::erased(error)),
    }
}
#[cold]
fn from_mlua(error: mlua::Error) -> ErrorMaybeBt {
    match error {
        mlua::Error::ExternalError(error) => from_arc_dyn(error),
        mlua::Error::WithContext { context, cause } => {
            let mut ebt = from_arc_mlua(cause);
            let chain = match &mut ebt {
                Ok(error) => &mut Arc::make_mut(&mut error.0).chain,
                Err(chain) => chain,
            };
            chain.context(Box::new(context), None);
            ebt
        }
        _ => from_arc_dyn_ignore_mlua(Arc::new(error)),
    }
}
#[cold]
fn from_arc_mlua(error: Arc<mlua::Error>) -> ErrorMaybeBt {
    match &*error {
        mlua::Error::ExternalError { .. } | mlua::Error::WithContext { .. } => {
            from_mlua(Arc::unwrap_or_clone(error))
        }
        _ => from_arc_dyn_ignore_mlua(error),
    }
}

#[derive(Clone)]
struct ErrorChain {
    error: Arc<dyn error::Error + Send + Sync>,
}
impl ErrorChain {
    fn erased(error: Arc<dyn error::Error + Send + Sync>) -> Self {
        Self { error }
    }
    fn context(
        &mut self,
        ctx: Box<dyn Display + Send + Sync>,
        loc: Option<&'static Location<'static>>,
    ) {
        struct ErrorContext {
            base: Arc<dyn error::Error + Send + Sync>,
            ctx: Box<dyn Display + Send + Sync>,
            loc: Option<&'static Location<'static>>,
        }
        impl Display for ErrorContext {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if let Some(loc) = self.loc {
                    writeln!(f, "(at {loc})")?;
                }
                write!(f, "{}", self.ctx)
            }
        }
        impl Debug for ErrorContext {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                Display::fmt(self, f)
            }
        }
        impl error::Error for ErrorContext {
            fn source(&self) -> Option<&(dyn error::Error + 'static)> {
                Some(&self.base)
            }
        }
        *self = ErrorChain {
            error: Arc::new(ErrorContext {
                base: self.error.clone(),
                ctx,
                loc,
            }),
        }
    }
}

impl error::Error for ErrorInner {}

impl From<Error> for mlua::Error {
    fn from(value: Error) -> Self {
        let Error(inner) = value;
        mlua::Error::ExternalError(inner)
    }
}
impl<E> From<E> for Error
where
    E: error::Error + Send + Sync + 'static,
{
    #[cold]
    fn from(value: E) -> Self {
        supply_backtrace!(from_arc_dyn(Arc::new(value)))
    }
}

fn downcast_arc_error<T: error::Error + 'static>(
    err: Arc<dyn error::Error + Send + Sync>,
) -> Result<Arc<T>, Arc<dyn error::Error + Send + Sync>> {
    if (*err).is::<T>() {
        // SAFETY: Imitates `Arc::downcast`
        unsafe {
            let ptr = Arc::into_raw(err);
            Ok(Arc::from_raw(ptr.cast()))
        }
    } else {
        Err(err)
    }
}

impl Display for ErrorInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}
impl Debug for ErrorInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let error = &self.chain.error;

        write!(f, "{error}")?;
        if let Some(cause) = error.source() {
            write!(f, "\n\nCaused by:")?;
            let multiple = cause.source().is_some();
            for (n, error) in std::iter::successors(Some(cause), |it| it.source()).enumerate() {
                writeln!(f)?;
                let mut indented = Indented {
                    inner: f,
                    number: if multiple { Some(n) } else { None },
                    started: false,
                };
                write!(indented, "{}", error)?;
            }
        }
        write!(f, "\n\nStack backtrace:\n{}", self.backtrace)?;
        Ok(())
    }
}
struct Indented<'a, 'b> {
    inner: &'a mut fmt::Formatter<'b>,
    number: Option<usize>,
    started: bool,
}

impl Write for Indented<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for (i, line) in s.split('\n').enumerate() {
            if !self.started {
                self.started = true;
                match self.number {
                    Some(number) => write!(self.inner, "{: >5}: ", number)?,
                    None => self.inner.write_str("    ")?,
                }
            } else if i > 0 {
                self.inner.write_char('\n')?;
                if self.number.is_some() {
                    self.inner.write_str("       ")?;
                } else {
                    self.inner.write_str("    ")?;
                }
            }

            self.inner.write_str(line)?;
        }

        Ok(())
    }
}
