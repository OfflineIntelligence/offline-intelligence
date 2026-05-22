const axios = require('axios');

const VERSION = '0.1.6';

class Config {
    constructor() {
        this.modelPath = 'default.gguf';
        this.llamaBin = 'llama-server';
        this.llamaHost = '127.0.0.1';
        this.llamaPort = 8081;
        this.backendUrl = 'http://127.0.0.1:8081';
        this.openrouterApiKey = '';
        this.ctxSize = 8192;
        this.batchSize = 256;
        this.threads = 6;
        this.gpuLayers = 20;
        this.healthTimeoutSeconds = 60;
        this.hotSwapGraceSeconds = 25;
        this.maxConcurrentStreams = 4;
        this.prometheusPort = 9000;
        this.apiHost = '127.0.0.1';
        this.apiPort = 9999;
        this.requestsPerSecond = 24;
        this.generateTimeoutSeconds = 300;
        this.streamTimeoutSeconds = 600;
        this.healthCheckTimeoutSeconds = 90;
        this.queueSize = 100;
        this.queueTimeoutSeconds = 30;
    }

    static fromEnv() {
        const config = new Config();
        if (process.env.MODEL_PATH) config.modelPath = process.env.MODEL_PATH;
        if (process.env.LLAMA_BIN) config.llamaBin = process.env.LLAMA_BIN;
        if (process.env.LLAMA_HOST) config.llamaHost = process.env.LLAMA_HOST;
        if (process.env.LLAMA_PORT) config.llamaPort = parseInt(process.env.LLAMA_PORT, 10);
        if (process.env.BACKEND_URL) config.backendUrl = process.env.BACKEND_URL;
        if (process.env.OPENROUTER_API_KEY) config.openrouterApiKey = process.env.OPENROUTER_API_KEY;
        if (process.env.API_HOST) config.apiHost = process.env.API_HOST;
        if (process.env.API_PORT) config.apiPort = parseInt(process.env.API_PORT, 10);
        if (process.env.CTX_SIZE) config.ctxSize = parseInt(process.env.CTX_SIZE, 10);
        if (process.env.GPU_LAYERS) config.gpuLayers = parseInt(process.env.GPU_LAYERS, 10);
        return config;
    }
}

class OfflineIntelligence {
    constructor(config) {
        this.config = config || new Config();
        this.baseUrl = `http://${this.config.apiHost}:${this.config.apiPort}`;
    }

    async healthCheck() {
        try {
            const response = await axios.get(`${this.baseUrl}/healthz`);
            return response.data;
        } catch (error) {
            throw new Error(`Health check failed: ${error.message}`);
        }
    }

    async getStatus() {
        try {
            const response = await axios.get(`${this.baseUrl}/admin/status`);
            return response.data;
        } catch (error) {
            throw new Error(`Status check failed: ${error.message}`);
        }
    }

    async loadModel(modelPath) {
        try {
            const response = await axios.post(`${this.baseUrl}/admin/load`, {
                model_path: modelPath
            });
            return response.data;
        } catch (error) {
            throw new Error(`Model loading failed: ${error.message}`);
        }
    }

    async stopModel() {
        try {
            const response = await axios.post(`${this.baseUrl}/admin/stop`);
            return response.data;
        } catch (error) {
            throw new Error(`Model stop failed: ${error.message}`);
        }
    }

    async generate(prompt, options = {}) {
        try {
            const response = await axios.post(`${this.baseUrl}/generate`, {
                prompt,
                ...options
            });
            return response.data;
        } catch (error) {
            throw new Error(`Generation failed: ${error.message}`);
        }
    }

    async generateStream(prompt, options = {}) {
        try {
            const response = await axios.post(`${this.baseUrl}/generate/stream`, {
                prompt,
                ...options
            }, {
                responseType: 'stream'
            });
            return response.data;
        } catch (error) {
            throw new Error(`Stream generation failed: ${error.message}`);
        }
    }

    async getConversations() {
        try {
            const response = await axios.get(`${this.baseUrl}/conversations`);
            return response.data;
        } catch (error) {
            throw new Error(`Failed to list conversations: ${error.message}`);
        }
    }

    async getConversation(id) {
        try {
            const response = await axios.get(`${this.baseUrl}/conversations/${id}`);
            return response.data;
        } catch (error) {
            throw new Error(`Failed to get conversation: ${error.message}`);
        }
    }

    async deleteConversation(id) {
        try {
            const response = await axios.delete(`${this.baseUrl}/conversations/${id}`);
            return response.data;
        } catch (error) {
            throw new Error(`Failed to delete conversation: ${error.message}`);
        }
    }

    async getConversationTitle(id) {
        try {
            const response = await axios.get(`${this.baseUrl}/conversations/${id}/title`);
            return response.data;
        } catch (error) {
            throw new Error(`Failed to get conversation title: ${error.message}`);
        }
    }

    async generateTitle(sessionId, firstMessage) {
        try {
            const response = await axios.post(`${this.baseUrl}/generate/title`, {
                session_id: sessionId,
                first_message: firstMessage
            });
            return response.data;
        } catch (error) {
            throw new Error(`Failed to generate title: ${error.message}`);
        }
    }

    async getMemoryStats(sessionId) {
        try {
            const response = await axios.get(`${this.baseUrl}/memory/stats/${sessionId}`);
            return response.data;
        } catch (error) {
            throw new Error(`Failed to get memory stats: ${error.message}`);
        }
    }

    async optimizeMemory() {
        try {
            const response = await axios.post(`${this.baseUrl}/memory/optimize`);
            return response.data;
        } catch (error) {
            throw new Error(`Memory optimization failed: ${error.message}`);
        }
    }

    async cleanupMemory() {
        try {
            const response = await axios.post(`${this.baseUrl}/memory/cleanup`);
            return response.data;
        } catch (error) {
            throw new Error(`Memory cleanup failed: ${error.message}`);
        }
    }

    static version() {
        return VERSION;
    }
}

module.exports = {
    Config,
    OfflineIntelligence,
    version: OfflineIntelligence.version
};
