//! Appel de Ghostscript en tant que bibliothèque (API `gsapi_*` de libgs),
//! sans lancer de processus externe.

use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::sync::Mutex;

#[repr(C)]
struct GsapiRevision {
    product: *const c_char,
    copyright: *const c_char,
    revision: c_long,
    revisiondate: c_long,
}

type StdinFn = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type OutFn = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;

extern "C" {
    fn gsapi_revision(pr: *mut GsapiRevision, len: c_int) -> c_int;
    fn gsapi_new_instance(pinstance: *mut *mut c_void, caller_handle: *mut c_void) -> c_int;
    fn gsapi_delete_instance(instance: *mut c_void);
    fn gsapi_set_arg_encoding(instance: *mut c_void, encoding: c_int) -> c_int;
    fn gsapi_set_stdio_with_handle(
        instance: *mut c_void,
        stdin_fn: Option<StdinFn>,
        stdout_fn: Option<OutFn>,
        stderr_fn: Option<OutFn>,
        caller_handle: *mut c_void,
    ) -> c_int;
    fn gsapi_init_with_args(instance: *mut c_void, argc: c_int, argv: *mut *mut c_char) -> c_int;
    fn gsapi_exit(instance: *mut c_void) -> c_int;
}

const GS_ARG_ENCODING_UTF8: c_int = 1;
/// Code renvoyé lorsque l'interpréteur s'arrête normalement (`quit`).
const GS_ERROR_QUIT: c_int = -101;

/// Selon sa compilation, libgs n'accepte qu'une instance à la fois :
/// les compressions sont donc exécutées l'une après l'autre.
static GS_LOCK: Mutex<()> = Mutex::new(());

/// Version de la bibliothèque Ghostscript liée, par exemple « 10.05.1 ».
pub fn version() -> Option<String> {
    let mut rev = GsapiRevision {
        product: std::ptr::null(),
        copyright: std::ptr::null(),
        revision: 0,
        revisiondate: 0,
    };
    let ok = unsafe { gsapi_revision(&mut rev, std::mem::size_of::<GsapiRevision>() as c_int) } == 0;
    let r = rev.revision;
    (ok && r > 0).then(|| format!("{}.{:02}.{}", r / 1000, (r / 10) % 100, r % 10))
}

unsafe extern "C" fn no_stdin(_: *mut c_void, _: *mut c_char, _: c_int) -> c_int {
    0
}

/// Accumule les sorties de Ghostscript (utile pour diagnostiquer un échec).
unsafe extern "C" fn capture(handle: *mut c_void, s: *const c_char, len: c_int) -> c_int {
    if !handle.is_null() && !s.is_null() && len > 0 {
        let buf = &mut *(handle as *mut Vec<u8>);
        if buf.len() < 64 * 1024 {
            buf.extend_from_slice(std::slice::from_raw_parts(s as *const u8, len as usize));
        }
    }
    len
}

/// Exécute Ghostscript avec ces arguments (sans le nom de programme).
/// Renvoie les messages produits, en cas de succès comme d'échec.
pub fn run(args: &[String]) -> Result<String, String> {
    let c_args: Vec<CString> = std::iter::once("gs")
        .chain(args.iter().map(String::as_str))
        .map(|a| CString::new(a).map_err(|_| "Argument invalide (caractère nul).".to_string()))
        .collect::<Result<_, _>>()?;
    let mut argv: Vec<*mut c_char> = c_args.iter().map(|a| a.as_ptr() as *mut c_char).collect();

    let _guard = GS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut output: Box<Vec<u8>> = Box::default();
    let handle = &mut *output as *mut Vec<u8> as *mut c_void;

    let code = unsafe {
        let mut instance: *mut c_void = std::ptr::null_mut();
        let code = gsapi_new_instance(&mut instance, handle);
        if code < 0 || instance.is_null() {
            return Err(format!("Impossible d'initialiser Ghostscript (code {code})."));
        }
        gsapi_set_stdio_with_handle(instance, Some(no_stdin), Some(capture), Some(capture), handle);
        gsapi_set_arg_encoding(instance, GS_ARG_ENCODING_UTF8);
        let mut code = gsapi_init_with_args(instance, argv.len() as c_int, argv.as_mut_ptr());
        let exit = gsapi_exit(instance);
        if code == 0 || code == GS_ERROR_QUIT {
            code = exit;
        }
        gsapi_delete_instance(instance);
        code
    };

    let text = String::from_utf8_lossy(&output).into_owned();
    if code == 0 || code == GS_ERROR_QUIT {
        Ok(text)
    } else {
        Err(text)
    }
}
