use neon::prelude::*;
use rayon::prelude::*;
use serde_hkx_for_gui::convert;
use serde_hkx_for_gui::par_walk_dir::{load_dir_node, DirEntry};
use serde_hkx_for_gui::status::{Payload, Status};
use std::sync::Arc;

use crate::get_tokio_rt;

/// Module export
pub fn register_api(cx: &mut ModuleContext) -> NeonResult<()> {
    cx.export_function("loadDirNode", js_load_dir_node)?;
    cx.export_function("convert", js_convert)?;
    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

fn js_load_dir_node(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let dirs_js = cx.argument::<JsArray>(0)?;
    let mut dirs = Vec::new();
    for i in 0..dirs_js.len(&mut cx) {
        let s = dirs_js
            .get::<JsString, _, _>(&mut cx, i)
            .or_else(|_| cx.throw_type_error("dirs must be array of strings"))?;
        dirs.push(s.value(&mut cx));
    }

    let channel = Arc::new(cx.channel());
    let (deferred, promise) = cx.promise();

    std::thread::spawn(move || {
        let channel = channel.clone();
        let res = load_dir_node(dirs);

        deferred.settle_with(&channel, move |mut cx| match res {
            Ok(items) => {
                let js_array = JsArray::new(&mut cx, items.len());
                for (i, item) in items.iter().enumerate() {
                    let js_item = tree_item_to_js(&mut cx, item)?;
                    js_array.set(&mut cx, i as u32, js_item)?;
                }
                Ok(js_array)
            }
            Err(errs) => {
                let err = errs
                    .par_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join("\n");

                cx.throw_error(err)
            }
        });
    });

    Ok(promise)
}

fn tree_item_to_js<'a, C: Context<'a>>(cx: &mut C, item: &DirEntry) -> JsResult<'a, JsObject> {
    let id = cx.string(&item.name);
    let label = cx.string(&item.path);

    let js_obj = cx.empty_object();
    js_obj.set(cx, "id", id)?;
    js_obj.set(cx, "label", label)?;

    if let Some(children) = &item.children {
        let js_children = JsArray::new(cx, children.len());
        for (i, child) in children.iter().enumerate() {
            let js_child = tree_item_to_js(cx, child)?;
            js_children.set(cx, i as u32, js_child)?;
        }
        js_obj.set(cx, "children", js_children)?;
    }

    Ok(js_obj)
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// ```ts
/// function convert(inputs: string[], output: string, format: OutputFormat, roots?: string[]): Promise<void>;
/// ```
fn js_convert(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let inputs = cx.argument::<JsArray>(0)?;
    let mut inputs_vec = Vec::new();
    for i in 0..inputs.len(&mut cx) {
        let js_str = inputs
            .get::<JsString, _, _>(&mut cx, i)
            .or_else(|_| cx.throw_type_error("inputs must be array of strings"))?;
        inputs_vec.push(js_str.value(&mut cx));
    }

    // output
    let output = match cx.argument_opt(1) {
        Some(arg) => Some(
            arg.downcast::<JsString, _>(&mut cx)
                .or_else(|_| cx.throw_type_error("output must be a string"))?
                .value(&mut cx),
        ),
        None => None,
    };

    // format
    let format = cx.argument::<JsString>(2)?.value(&mut cx);

    // roots (optional)
    let roots = match cx.argument_opt(3) {
        Some(arg) => {
            let arr = arg
                .downcast::<JsArray, _>(&mut cx)
                .or_else(|_| cx.throw_type_error("roots must be array of strings"))?;
            let mut vec = Vec::new();
            for i in 0..arr.len(&mut cx) {
                let js_str = arr
                    .get::<JsString, _, _>(&mut cx, i)
                    .or_else(|_| cx.throw_type_error("roots elements must be strings"))?;
                vec.push(js_str.value(&mut cx));
            }
            Some(vec)
        }
        None => None,
    };

    // progress callback
    let progress_fn = Arc::new(cx.argument::<JsFunction>(4)?.root(&mut cx));
    let channel = cx.channel();
    let (deferred, promise) = cx.promise();

    get_tokio_rt(&mut cx)?.spawn(async move {
        let cloned_channel = channel.clone();
        let status_sender = move |payload: Payload| {
            let progress_fn = progress_fn.clone();
            cloned_channel.send(move |mut cx| {
                let js_payload = payload_to_js(&mut cx, &payload)?;
                progress_fn
                    .to_inner(&mut cx)
                    .bind(&mut cx)
                    .arg(js_payload)?
                    .call::<()>()?;
                Ok(())
            });
        };

        let res = convert(inputs_vec, output, &format, roots, status_sender).await;

        deferred.settle_with(&channel, move |mut cx| match res {
            Ok(_) => Ok(cx.undefined()),
            Err(err) => cx.throw_error(format!("{:?}", err)),
        });
    });

    Ok(promise)
}

/// Rust Status -> JsNumber
fn status_to_js<'a, C: Context<'a>>(cx: &mut C, status: Status) -> JsResult<'a, JsNumber> {
    Ok(cx.number(status as u8))
}

/// Rust Payload -> JsObject
fn payload_to_js<'a, C: Context<'a>>(cx: &mut C, payload: &Payload) -> JsResult<'a, JsObject> {
    let path_id = cx.number(payload.path_id as f64);
    let status = status_to_js(cx, payload.status)?;

    let js_obj = cx.empty_object();
    js_obj.set(cx, "pathId", path_id)?;
    js_obj.set(cx, "status", status)?;

    Ok(js_obj)
}
