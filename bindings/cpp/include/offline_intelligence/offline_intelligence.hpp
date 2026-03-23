#ifndef OFFLINE_INTELLIGENCE_HPP
#define OFFLINE_INTELLIGENCE_HPP

/**
 * offline_intelligence.hpp
 * C++ HTTP client for the Offline Intelligence server.
 * Version: 0.1.3
 *
 * Requires cpp-httplib (https://github.com/yhirose/cpp-httplib)
 * Add it via CMake FetchContent or place httplib.h alongside this header.
 *
 * Usage:
 *   offline_intelligence::Config cfg;
 *   cfg.api_port = 9999;
 *   offline_intelligence::OfflineIntelligence ai(cfg);
 *   auto resp = ai.health_check();
 */

#include <httplib.h>
#include <nlohmann/json.hpp>

#include <string>
#include <cstdint>
#include <functional>
#include <sstream>
#include <stdexcept>

namespace offline_intelligence {

using json = nlohmann::json;

// ── Exception ───────────────────────────────────────────────────────────

class OfflineIntelligenceException : public std::runtime_error {
public:
    int status_code;
    explicit OfflineIntelligenceException(const std::string& msg, int code = 0)
        : std::runtime_error(msg), status_code(code) {}
};

// ── Config ───────────────────────────────────────────────────────────────

struct Config {
    std::string model_path                = "default.gguf";
    std::string llama_bin                 = "llama-server";
    std::string llama_host                = "127.0.0.1";
    uint16_t    llama_port                = 8081;
    std::string backend_url               = "http://127.0.0.1:8081";
    std::string openrouter_api_key        = "";
    uint32_t    ctx_size                  = 8192;
    uint32_t    batch_size                = 256;
    uint32_t    threads                   = 6;
    uint32_t    gpu_layers                = 20;
    uint64_t    health_timeout_seconds    = 60;
    uint64_t    hot_swap_grace_seconds    = 25;
    uint32_t    max_concurrent_streams    = 4;
    uint16_t    prometheus_port           = 9000;
    std::string api_host                  = "127.0.0.1";
    uint16_t    api_port                  = 9999;
    uint32_t    requests_per_second       = 24;
    uint64_t    generate_timeout_seconds  = 300;
    uint64_t    stream_timeout_seconds    = 600;
    uint64_t    health_check_timeout_secs = 90;
    size_t      queue_size                = 100;
    uint64_t    queue_timeout_seconds     = 30;

    /** Populate from environment variables. */
    static Config from_env() {
        Config cfg;
        auto env = [](const char* k) -> std::string {
            const char* v = std::getenv(k);
            return v ? std::string(v) : std::string{};
        };
        if (auto v = env("MODEL_PATH");           !v.empty()) cfg.model_path = v;
        if (auto v = env("LLAMA_BIN");            !v.empty()) cfg.llama_bin = v;
        if (auto v = env("LLAMA_HOST");           !v.empty()) cfg.llama_host = v;
        if (auto v = env("LLAMA_PORT");           !v.empty()) cfg.llama_port = static_cast<uint16_t>(std::stoi(v));
        if (auto v = env("BACKEND_URL");          !v.empty()) cfg.backend_url = v;
        if (auto v = env("OPENROUTER_API_KEY");   !v.empty()) cfg.openrouter_api_key = v;
        if (auto v = env("API_HOST");             !v.empty()) cfg.api_host = v;
        if (auto v = env("API_PORT");             !v.empty()) cfg.api_port = static_cast<uint16_t>(std::stoi(v));
        if (auto v = env("CTX_SIZE");             !v.empty()) cfg.ctx_size = static_cast<uint32_t>(std::stoi(v));
        if (auto v = env("GPU_LAYERS");           !v.empty()) cfg.gpu_layers = static_cast<uint32_t>(std::stoi(v));
        return cfg;
    }
};

// ── Main Client ──────────────────────────────────────────────────────────

class OfflineIntelligence {
public:
    explicit OfflineIntelligence(const Config& config = Config{})
        : cfg_(config)
        , cli_(config.api_host, config.api_port)
    {
        cli_.set_connection_timeout(static_cast<time_t>(config.health_timeout_seconds), 0);
        cli_.set_read_timeout(static_cast<time_t>(config.generate_timeout_seconds), 0);
        cli_.set_write_timeout(30, 0);
    }

    static OfflineIntelligence from_env() {
        return OfflineIntelligence(Config::from_env());
    }

    // ── Health & Status ──────────────────────────────────────────────

    /** GET /healthz */
    json health_check() { return get_json("/healthz"); }

    /** GET /admin/status */
    json get_status()   { return get_json("/admin/status"); }

    // ── Model Management ──────────────────────────────────────────

    /** POST /admin/load */
    json load_model(const std::string& model_path) {
        json body = {{"model_path", model_path}};
        return post_json("/admin/load", body);
    }

    /** POST /admin/stop */
    json stop_model() { return post_json("/admin/stop", json::object()); }

    // ── Generation ──────────────────────────────────────────────────

    /** POST /generate */
    json generate(const std::string& prompt, json extra = json::object()) {
        json body = extra;
        body["prompt"] = prompt;
        return post_json("/generate", body);
    }

    /**
     * POST /generate/stream — streams SSE, invoking callback for each token.
     * @param prompt   The prompt text.
     * @param on_chunk Called for each received text chunk.
     */
    void generate_stream(const std::string& prompt,
                         std::function<void(const std::string&)> on_chunk,
                         json extra = json::object())
    {
        json body = extra;
        body["prompt"] = prompt;
        std::string body_str = body.dump();

        cli_.set_read_timeout(static_cast<time_t>(cfg_.stream_timeout_seconds), 0);

        // Use httplib's streaming POST with ContentReceiver
        std::string accumulated;
        auto res = cli_.Post("/generate/stream",
            httplib::Headers{{"Accept", "text/event-stream"}},
            body_str,
            "application/json");

        if (!res) {
            throw OfflineIntelligenceException(
                "Stream request failed: " + httplib::to_string(res.error()));
        }
        if (res->status >= 400) {
            throw OfflineIntelligenceException("HTTP " + std::to_string(res->status), res->status);
        }

        // Parse the full SSE body (for non-chunked servers)
        std::istringstream ss(res->body);
        std::string line;
        while (std::getline(ss, line)) {
            // Strip trailing \r
            if (!line.empty() && line.back() == '\r') line.pop_back();
            if (line.size() >= 6 && line.substr(0, 6) == "data: ") {
                std::string payload = line.substr(6);
                if (payload == "[DONE]") break;
                try {
                    auto j = json::parse(payload);
                    if (j.contains("choices")) {
                        auto content = j["choices"][0]["delta"].value("content", std::string{});
                        if (!content.empty()) on_chunk(content);
                    } else if (j.contains("text")) {
                        on_chunk(j["text"].get<std::string>());
                    }
                } catch (...) {
                    on_chunk(payload);
                }
            }
        }

        cli_.set_read_timeout(static_cast<time_t>(cfg_.generate_timeout_seconds), 0);
    }

    // ── Conversations ──────────────────────────────────────────────

    /** GET /conversations */
    json get_conversations() { return get_json("/conversations"); }

    /** GET /conversations/{id} */
    json get_conversation(const std::string& id) {
        return get_json("/conversations/" + id);
    }

    /** DELETE /conversations/{id} */
    json delete_conversation(const std::string& id) {
        auto res = cli_.Delete("/conversations/" + id);
        return handle_response(res);
    }

    /** GET /conversations/{id}/title */
    json get_conversation_title(const std::string& id) {
        return get_json("/conversations/" + id + "/title");
    }

    /** POST /generate/title */
    json generate_title(const std::string& session_id, const std::string& first_message) {
        json body = {{"session_id", session_id}, {"first_message", first_message}};
        return post_json("/generate/title", body);
    }

    // ── Memory ────────────────────────────────────────────────────────────

    /** GET /memory/stats/{session_id} */
    json get_memory_stats(const std::string& session_id) {
        return get_json("/memory/stats/" + session_id);
    }

    /** POST /memory/optimize */
    json optimize_memory() { return post_json("/memory/optimize", json::object()); }

    /** POST /memory/cleanup */
    json cleanup_memory()  { return post_json("/memory/cleanup",  json::object()); }

    // ── Version ────────────────────────────────────────────────────────────

    static std::string version() { return "0.1.3"; }

    const Config& config() const { return cfg_; }

private:
    Config        cfg_;
    httplib::Client cli_;

    json get_json(const std::string& path) {
        auto res = cli_.Get(path);
        return handle_response(res);
    }

    json post_json(const std::string& path, const json& body) {
        auto res = cli_.Post(path, body.dump(), "application/json");
        return handle_response(res);
    }

    template<typename R>
    json handle_response(const R& res) {
        if (!res) {
            throw OfflineIntelligenceException(
                "Request failed: " + httplib::to_string(res.error()));
        }
        if (res->status >= 400) {
            throw OfflineIntelligenceException(
                "HTTP " + std::to_string(res->status) + ": " + res->body,
                res->status);
        }
        try {
            return json::parse(res->body);
        } catch (...) {
            return json{{"raw", res->body}};
        }
    }
};

// ── Legacy Server shim (backward compatibility) ────────────────────────────

class Server {
public:
    [[deprecated("Use OfflineIntelligence instead.")]]
    static bool run_server(const Config& config) {
        OfflineIntelligence ai(config);
        ai.health_check();
        return true;
    }
    static std::string version() { return OfflineIntelligence::version(); }
};

} // namespace offline_intelligence

#endif // OFFLINE_INTELLIGENCE_HPP