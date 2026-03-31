#include <napi.h>
#include <iostream>
#include <string>

namespace offline_intelligence {
    struct Config {
        std::string model_path;
        std::string llama_bin;
        std::string llama_host;
        uint16_t llama_port;
        uint32_t ctx_size;
        uint32_t batch_size;
        uint32_t threads;
        uint32_t gpu_layers;
        uint64_t health_timeout_seconds;
        uint64_t hot_swap_grace_seconds;
        uint32_t max_concurrent_streams;
        uint16_t prometheus_port;
        std::string api_host;
        uint16_t api_port;
        uint32_t requests_per_second;
        uint64_t generate_timeout_seconds;
        uint64_t stream_timeout_seconds;
        uint64_t health_check_timeout_seconds;
        size_t queue_size;
        uint64_t queue_timeout_seconds;
    };

    Config Config_from_env() {
        Config cfg;
        cfg.model_path = "default.gguf";
        cfg.llama_bin = "llama-server";
        cfg.llama_host = "127.0.0.1";
        cfg.llama_port = 8081;
        cfg.ctx_size = 8192;
        cfg.batch_size = 256;
        cfg.threads = 6;
        cfg.gpu_layers = 20;
        cfg.health_timeout_seconds = 60;
        cfg.hot_swap_grace_seconds = 25;
        cfg.max_concurrent_streams = 4;
        cfg.prometheus_port = 9000;
        cfg.api_host = "127.0.0.1";
        cfg.api_port = 8000;
        cfg.requests_per_second = 24;
        cfg.generate_timeout_seconds = 300;
        cfg.stream_timeout_seconds = 600;
        cfg.health_check_timeout_seconds = 90;
        cfg.queue_size = 100;
        cfg.queue_timeout_seconds = 30;
        return cfg;
    }

    bool run_server(const Config& cfg) {
        std::cout << "Starting Offline Intelligence server..." << std::endl;
        std::cout << "API Server: " << cfg.api_host << ":" << cfg.api_port << std::endl;
        std::cout << "LLM Backend: " << cfg.llama_host << ":" << cfg.llama_port << std::endl;
        return true;
    }
}

Napi::Object Init(Napi::Env env, Napi::Object exports) {
    
    auto configClass = Napi::Object::New(env);
    configClass.Set("fromEnv", Napi::Function::New(env, [](const Napi::CallbackInfo& info) {
        auto cfg = offline_intelligence::Config_from_env();
        
        auto obj = Napi::Object::New(info.Env());
        obj.Set("modelPath", cfg.model_path);
        obj.Set("llamaBin", cfg.llama_bin);
        obj.Set("llamaHost", cfg.llama_host);
        obj.Set("llamaPort", cfg.llama_port);
        obj.Set("ctxSize", cfg.ctx_size);
        obj.Set("batchSize", cfg.batch_size);
        obj.Set("threads", cfg.threads);
        obj.Set("gpuLayers", cfg.gpu_layers);
        obj.Set("apiHost", cfg.api_host);
        obj.Set("apiPort", cfg.api_port);
        
        return obj;
    }));
    
    exports.Set("Config", configClass);
    
    exports.Set("runServer", Napi::Function::New(env, [](const Napi::CallbackInfo& info) {
        if (info.Length() < 1 || !info[0].IsObject()) {
            Napi::TypeError::New(info.Env(), "Config object required").ThrowAsJavaScriptException();
            return info.Env().Undefined();
        }
        
        auto configObj = info[0].As<Napi::Object>();
        offline_intelligence::Config cfg;
        
        cfg.model_path = configObj.Get("modelPath").ToString().Utf8Value();
        cfg.llama_bin = configObj.Get("llamaBin").ToString().Utf8Value();
        cfg.llama_host = configObj.Get("llamaHost").ToString().Utf8Value();
        cfg.llama_port = configObj.Get("llamaPort").ToNumber().Uint32Value();
        cfg.ctx_size = configObj.Get("ctxSize").ToNumber().Uint32Value();
        cfg.batch_size = configObj.Get("batchSize").ToNumber().Uint32Value();
        cfg.threads = configObj.Get("threads").ToNumber().Uint32Value();
        cfg.gpu_layers = configObj.Get("gpuLayers").ToNumber().Uint32Value();
        cfg.api_host = configObj.Get("apiHost").ToString().Utf8Value();
        cfg.api_port = configObj.Get("apiPort").ToNumber().Uint32Value();
        
        bool result = offline_intelligence::run_server(cfg);
        return Napi::Boolean::New(info.Env(), result);
    }));
    
    exports.Set("version", Napi::Function::New(env, [](const Napi::CallbackInfo& info) {
        return Napi::String::New(info.Env(), "0.1.0");
    }));
    
    return exports;
}

NODE_API_MODULE(offline_intelligence, Init)
