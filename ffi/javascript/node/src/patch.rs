use nemesis_merge::{behavior_gen, Config, Status};
use neon::prelude::*;
use std::path::PathBuf;

use crate::get_tokio_rt;
use mod_info::{GetModsInfo as _, ModInfo, ModsInfo};
use skyrim_data_dir::Runtime;

/// Module export
pub fn register_api(cx: &mut ModuleContext) -> NeonResult<()> {
    cx.export_function("behaviorGen", js_behavior_gen)?;
    cx.export_function("loadModsInfo", js_load_mods_info)?;
    cx.export_function("getSkyrimDataDir", js_get_skyrim_data_dir)?;
    Ok(())
}

fn js_behavior_gen(mut cx: FunctionContext) -> JsResult<JsPromise> {
    // 1. argument: ids (string[])
    let js_ids = cx.argument::<JsArray>(0)?;
    let mut ids: Vec<PathBuf> = Vec::with_capacity(js_ids.len(&mut cx) as usize);

    for i in 0..js_ids.len(&mut cx) {
        let js_str = js_ids.get::<JsString, _, _>(&mut cx, i)?;
        ids.push(PathBuf::from(js_str.value(&mut cx)));
    }

    // 2. argument: config (object)
    let js_config = cx.argument::<JsObject>(1)?;
    let config = js_object_to_config(&mut cx, js_config)?;

    let channel = cx.channel();
    let (deferred, promise) = cx.promise();

    get_tokio_rt(&mut cx)?.spawn(async move {
        let result = behavior_gen(ids, config).await;

        deferred.settle_with(&channel, move |mut cx| match result {
            Ok(()) => Ok(cx.undefined()),
            Err(err) => cx.throw_error(err.to_string()),
        });
    });

    Ok(promise)
}

/// Convert JS object to Rust Config
fn js_object_to_config(cx: &mut FunctionContext, obj: Handle<JsObject>) -> NeonResult<Config> {
    use nemesis_merge::{DebugOptions, HackOptions, OutPutTarget, Status};

    // resourceDir: string
    let resource_dir: String = obj.get::<JsString, _, _>(cx, "resourceDir")?.value(cx);

    // outputDir: string
    let output_dir: String = obj.get::<JsString, _, _>(cx, "outputDir")?.value(cx);

    // outputTarget: "SkyrimSE" | "SkyrimLE"
    let output_target: String = obj.get::<JsString, _, _>(cx, "outputTarget")?.value(cx);
    let output_target = match output_target.as_str() {
        "SkyrimSE" => OutPutTarget::SkyrimSe,
        "SkyrimLE" => OutPutTarget::SkyrimLe,
        _ => OutPutTarget::default(),
    };

    // hackOptions?: { castRagdollEvent: boolean }
    let hack_options = if let Ok(js_hack) = obj.get::<JsObject, _, _>(cx, "hackOptions") {
        let cast_ragdoll_event: bool = js_hack
            .get::<JsBoolean, _, _>(cx, "castRagdollEvent")?
            .value(cx);
        Some(HackOptions { cast_ragdoll_event })
    } else {
        None
    };

    // debug: { outputPatchJson: boolean, outputMergedJson: boolean, outputMergedXml: boolean }
    let js_debug = obj.get::<JsObject, _, _>(cx, "debug")?;
    let debug = DebugOptions {
        output_patch_json: js_debug
            .get::<JsBoolean, _, _>(cx, "outputPatchJson")?
            .value(cx),
        output_merged_json: js_debug
            .get::<JsBoolean, _, _>(cx, "outputMergedJson")?
            .value(cx),
        output_merged_xml: js_debug
            .get::<JsBoolean, _, _>(cx, "outputMergedXml")?
            .value(cx),
    };

    // statusReport?: (status: Status) => void
    let status_report = if let Ok(func) = obj.get::<JsFunction, _, _>(cx, "statusReport") {
        let channel = cx.channel();
        let func_root = std::sync::Arc::new(func.root(cx));

        Some(Box::new(move |status: Status| {
            let func_root = func_root.clone();

            channel.send(move |mut cx| {
                let js_fn = func_root.to_inner(&mut cx);
                let js_status = status_to_js(&mut cx, &status)?;
                js_fn.bind(&mut cx).arg(js_status)?.call::<()>()
            });
        }) as Box<dyn Fn(Status) + Send + Sync>)
    } else {
        None
    };

    Ok(Config {
        resource_dir: PathBuf::from(resource_dir),
        output_dir: PathBuf::from(output_dir),
        output_target,
        status_report,
        hack_options,
        debug,
    })
}

/// Rust Status to JsObject
fn status_to_js<'a>(cx: &mut impl Context<'a>, status: &Status) -> JsResult<'a, JsObject> {
    let obj = cx.empty_object();

    match status {
        Status::ReadingPatches { index, total } => {
            let typ = cx.string("ReadingPatches");
            obj.set(cx, "type", typ)?;

            let index = cx.number(*index as f64);
            let total = cx.number(*total as f64);

            let content = cx.empty_object();
            content.set(cx, "index", index)?;
            content.set(cx, "total", total)?;
            obj.set(cx, "content", content)?;
        }
        Status::ParsingPatches { index, total } => {
            let typ = cx.string("ParsingPatches");
            obj.set(cx, "type", typ)?;

            let index = cx.number(*index as f64);
            let total = cx.number(*total as f64);

            let content = cx.empty_object();
            content.set(cx, "index", index)?;
            content.set(cx, "total", total)?;
            obj.set(cx, "content", content)?;
        }
        Status::ApplyingPatches { index, total } => {
            let typ = cx.string("ApplyingPatches");
            obj.set(cx, "type", typ)?;

            let index = cx.number(*index as f64);
            let total = cx.number(*total as f64);

            let content = cx.empty_object();
            content.set(cx, "index", index)?;
            content.set(cx, "total", total)?;
            obj.set(cx, "content", content)?;
        }
        Status::GeneratingHkxFiles { index, total } => {
            let typ = cx.string("GeneratingHkxFiles");
            obj.set(cx, "type", typ)?;

            let index = cx.number(*index as f64);
            let total = cx.number(*total as f64);

            let content = cx.empty_object();
            content.set(cx, "index", index)?;
            content.set(cx, "total", total)?;
            obj.set(cx, "content", content)?;
        }
        Status::Done => {
            let typ = cx.string("Done");

            let content = cx.undefined();
            obj.set(cx, "type", typ)?;
            obj.set(cx, "content", content)?;
        }
        Status::Error(msg) => {
            let typ = cx.string("Error");

            let content = cx.string(msg);
            obj.set(cx, "type", typ)?;
            obj.set(cx, "content", content)?;
        }
    }

    Ok(obj)
}

/// Neon wrapper for get_skyrim_data_dir
fn js_get_skyrim_data_dir(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let runtime_str = cx.argument::<JsString>(0)?.value(&mut cx);
    let runtime = match runtime_str.as_str() {
        "SkyrimSE" => Runtime::Se,
        "SkyrimLE" => Runtime::Le,
        _ => return cx.throw_error("Invalid runtime (must be SkyrimSE or SkyrimLE)"),
    };

    let channel = cx.channel();
    let (deferred, promise) = cx.promise();

    std::thread::spawn(move || {
        let res: Result<PathBuf, String> =
            { skyrim_data_dir::get_skyrim_data_dir(runtime).map_err(|err| format!("{err}")) };

        deferred.settle_with(&channel, move |mut cx| match res {
            Ok(path) => Ok(cx.string(path.display().to_string())),
            Err(err) => cx.throw_error(err),
        });
    });

    Ok(promise)
}

/// Neon wrapper for load_mods_info
fn js_load_mods_info(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let glob = cx.argument::<JsString>(0)?.value(&mut cx);

    let channel = cx.channel();
    let (deferred, promise) = cx.promise();

    std::thread::spawn(move || {
        let res: Result<Vec<ModInfo>, String> = (|| {
            let pattern = format!("{glob}/Nemesis_Engine/mod/*/info.ini");
            let info = ModsInfo::get_all(&pattern).map_err(|err| err.to_string())?;
            Ok(info)
        })();

        deferred.settle_with(&channel, move |mut cx| {
            match res {
                Ok(infos) => {
                    let arr = JsArray::new(&mut cx, infos.len());
                    for ModInfo {
                        id,
                        name,
                        author,
                        site,
                        auto: _,
                    } in infos
                    {
                        // ModInfo to JsObject
                        let id = cx.string(id);
                        let name = cx.string(name);
                        let author = cx.string(author);
                        let site = cx.string(site);

                        let obj = JsObject::new(&mut cx);
                        obj.set(&mut cx, "id", id)?;
                        obj.set(&mut cx, "name", name)?;
                        obj.set(&mut cx, "author", author)?;
                        obj.set(&mut cx, "site", site)?;
                    }
                    Ok(arr)
                }
                Err(err) => cx.throw_error(err),
            }
        });
    });

    Ok(promise)
}
