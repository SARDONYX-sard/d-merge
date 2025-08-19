use neon::prelude::*;
use std::path::Path;

pub fn register_api(cx: &mut ModuleContext) -> NeonResult<()> {
    cx.export_function("loggerInit", js_logger_init)?;
    cx.export_function("changeLogLevel", js_change_log_level)?;

    cx.export_function("logTrace", js_log_trace)?;
    cx.export_function("logDebug", js_log_debug)?;
    cx.export_function("logInfo", js_log_info)?;
    cx.export_function("logWarn", js_log_warn)?;
    cx.export_function("logError", js_log_error)?;
    Ok(())
}

/// Initialize logger
fn js_logger_init(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let log_dir = cx.argument::<JsString>(0)?.value(&mut cx);
    let log_name = cx.argument::<JsString>(1)?.value(&mut cx);

    let channel = cx.channel();
    let (deferred, promise) = cx.promise();

    std::thread::spawn(move || {
        let res =
            tracing_rotation::init(Path::new(&log_dir), &log_name).map_err(|e| format!("{e:?}"));

        deferred.settle_with(&channel, move |mut cx| match res {
            Ok(_) => Ok(cx.undefined()),
            Err(err) => cx.throw_error(err),
        });
    });

    Ok(promise)
}

/// Change log level
fn js_change_log_level(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let level = cx.argument::<JsString>(0)?.value(&mut cx);

    let channel = cx.channel();
    let (deferred, promise) = cx.promise();

    std::thread::spawn(move || {
        let res = tracing_rotation::change_level(&level).map_err(|e| format!("{e:?}"));

        deferred.settle_with(&channel, move |mut cx| match res {
            Ok(_) => Ok(cx.undefined()),
            Err(err) => cx.throw_error(err),
        });
    });

    Ok(promise)
}

fn js_log_trace(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let msg = cx.argument::<JsString>(0)?.value(&mut cx);
    tracing::trace!("{}", msg);
    Ok(cx.undefined())
}

fn js_log_debug(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let msg = cx.argument::<JsString>(0)?.value(&mut cx);
    tracing::debug!("{}", msg);
    Ok(cx.undefined())
}

fn js_log_info(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let msg = cx.argument::<JsString>(0)?.value(&mut cx);
    tracing::info!("{}", msg);
    Ok(cx.undefined())
}

fn js_log_warn(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let msg = cx.argument::<JsString>(0)?.value(&mut cx);
    tracing::warn!("{}", msg);
    Ok(cx.undefined())
}

fn js_log_error(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let msg = cx.argument::<JsString>(0)?.value(&mut cx);
    tracing::error!("{}", msg);
    Ok(cx.undefined())
}
