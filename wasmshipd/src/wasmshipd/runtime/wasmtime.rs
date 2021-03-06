use super::types::{
    FunctionExport, FunctionExports, RuntimeError, RuntimeResult, Value, ValueType,
};
use super::AbstractRuntime;
use wasmtime::*;

// To avoid Module of wasmtime
type SuperModule = super::types::Module;

pub struct Wasmtime {
    module: SuperModule,
    rt_module: Module,
    store: Store<()>,
    functions: FunctionExports,
}

// Implementation of the `AbstractRuntime` trait
impl AbstractRuntime for Wasmtime {
    fn new(super_module: SuperModule) -> RuntimeResult<Wasmtime> {
        let engine = Engine::default();
        let module_path = super_module.path.join(&super_module.main);
        let rt_module = Module::from_file(&engine, module_path).unwrap();
        let store = Store::new(&engine, ());
        Ok(Wasmtime {
            module: super_module,
            rt_module,
            store,
            functions: FunctionExports::new(),
        })
    }

    fn function_exports(&self) -> RuntimeResult<FunctionExports> {
        let mut func_exports = FunctionExports::new();
        let exports = self.rt_module.exports();
        for export in exports {
            if let Some(func) = export.ty().func() {
                let mut params = Vec::new();
                let mut results = Vec::new();
                for param in func.params() {
                    params.push(ValueType::from(param));
                }
                for result in func.results() {
                    results.push(ValueType::from(result));
                }
                func_exports.insert(
                    export.name().to_string(),
                    FunctionExport { params, results },
                );
            }
        }
        Ok(func_exports)
    }

    fn invoke(
        &mut self,
        function: Option<&str>,
        parameters: Vec<String>,
    ) -> RuntimeResult<Vec<Value>> {
        if function.is_none() && self.module.entry.is_none() {
            return Err(RuntimeError::NoEntryPoint);
        }
        let default_entry = self.module.entry.clone().unwrap();
        let function: &str = function.unwrap_or(&default_entry);
        if self.functions.is_empty() {
            self.functions = self.function_exports()?;
        }
        let functions = self.functions.clone();
        let function_export = match functions.get(function) {
            Some(function_export) => function_export,
            None => {
                return Err(RuntimeError::ExecutionError(format!(
                    "function {} not found",
                    function
                )))
            }
        };
        let function_params = function_export.params.clone();
        if parameters.len() != function_params.len() {
            return Err(RuntimeError::ExecutionError(format!(
                "function {} params count not match {}/{}",
                function,
                parameters.len(),
                function_params.len()
            )));
        }
        let function_results = function_export.results.clone();
        let instance = Instance::new(&mut self.store, &self.rt_module, &[])?;
        let func = instance.get_func(&mut self.store, function).unwrap();
        let mut params = vec![Val::null(); function_params.len()];
        let mut results = vec![Val::null(); function_results.len()];
        for (i, param) in function_params.iter().enumerate() {
            match param {
                ValueType::I32 => {
                    params[i] = Val::I32(parameters[i].parse()?);
                }
            }
        }
        match func.call(&mut self.store, &params, &mut results) {
            Ok(_) => {
                let mut values = Vec::new();
                for result in results {
                    match result {
                        Val::I32(x) => {
                            values.push(Value::I32(x));
                        }
                        _ => {
                            return Err(RuntimeError::ExecutionError(format!(
                                "function {} result type not match",
                                function
                            )));
                        }
                    }
                }
                Ok(values)
            }
            Err(err) => Err(RuntimeError::ExecutionError(err.to_string())),
        }
    }
}
