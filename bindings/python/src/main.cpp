#include <pybind11/pybind11.h>
#include <pybind11/stl.h>
#include <pybind11/functional.h>
#include <string>
#include <vector>
#include <memory>

// Forward declarations for the Rust library interface
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

    // Mock implementation - would call into actual Rust library
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
        // Mock implementation - would call actual Rust server
        printf("Starting Offline Intelligence server...\n");
        printf("API Server: %s:%d\n", cfg.api_host.c_str(), cfg.api_port);
        printf("LLM Backend: %s:%d\n", cfg.llama_host.c_str(), cfg.llama_port);
        return true;
    }
}

namespace py = pybind11;

PYBIND11_MODULE(offline_intelligence_py, m) {
    m.doc() = "Offline Intelligence Python Bindings";
    
    py::class_<offline_intelligence::Config>(m, "Config")
        .def(py::init<>())
        .def_readwrite("model_path", &offline_intelligence::Config::model_path)
        .def_readwrite("llama_bin", &offline_intelligence::Config::llama_bin)
        .def_readwrite("llama_host", &offline_intelligence::Config::llama_host)
        .def_readwrite("llama_port", &offline_intelligence::Config::llama_port)
        .def_readwrite("ctx_size", &offline_intelligence::Config::ctx_size)
        .def_readwrite("batch_size", &offline_intelligence::Config::batch_size)
        .def_readwrite("threads", &offline_intelligence::Config::threads)
        .def_readwrite("gpu_layers", &offline_intelligence::Config::gpu_layers)
        .def_readwrite("health_timeout_seconds", &offline_intelligence::Config::health_timeout_seconds)
        .def_readwrite("hot_swap_grace_seconds", &offline_intelligence::Config::hot_swap_grace_seconds)
        .def_readwrite("max_concurrent_streams", &offline_intelligence::Config::max_concurrent_streams)
        .def_readwrite("prometheus_port", &offline_intelligence::Config::prometheus_port)
        .def_readwrite("api_host", &offline_intelligence::Config::api_host)
        .def_readwrite("api_port", &offline_intelligence::Config::api_port)
        .def_readwrite("requests_per_second", &offline_intelligence::Config::requests_per_second)
        .def_readwrite("generate_timeout_seconds", &offline_intelligence::Config::generate_timeout_seconds)
        .def_readwrite("stream_timeout_seconds", &offline_intelligence::Config::stream_timeout_seconds)
        .def_readwrite("health_check_timeout_seconds", &offline_intelligence::Config::health_check_timeout_seconds)
        .def_readwrite("queue_size", &offline_intelligence::Config::queue_size)
        .def_readwrite("queue_timeout_seconds", &offline_intelligence::Config::queue_timeout_seconds)
        .def_static("from_env", &offline_intelligence::Config_from_env, 
                   "Create configuration from environment variables");
    
    m.def("run_server", &offline_intelligence::run_server, 
          "Start the Offline Intelligence server",
          py::arg("config"));
    
    m.def("version", []() { return "0.1.0"; }, 
          "Get library version");
}