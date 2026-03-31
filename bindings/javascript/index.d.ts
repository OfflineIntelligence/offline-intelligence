declare class Config {
    modelPath: string;
    llamaBin: string;
    llamaHost: string;
    llamaPort: number;
    backendUrl: string;
    openrouterApiKey: string;
    ctxSize: number;
    batchSize: number;
    threads: number;
    gpuLayers: number;
    healthTimeoutSeconds: number;
    hotSwapGraceSeconds: number;
    maxConcurrentStreams: number;
    prometheusPort: number;
    apiHost: string;
    apiPort: number;
    requestsPerSecond: number;
    generateTimeoutSeconds: number;
    streamTimeoutSeconds: number;
    healthCheckTimeoutSeconds: number;
    queueSize: number;
    queueTimeoutSeconds: number;

    static fromEnv(): Config;
}

declare class OfflineIntelligence {
    constructor(config?: Config);

    healthCheck(): Promise<any>;
    getStatus(): Promise<any>;

    loadModel(modelPath: string): Promise<any>;
    stopModel(): Promise<any>;

    generate(prompt: string, options?: Record<string, any>): Promise<any>;
    generateStream(prompt: string, options?: Record<string, any>): Promise<any>;

    getConversations(): Promise<any>;
    getConversation(id: string): Promise<any>;
    deleteConversation(id: string): Promise<any>;
    getConversationTitle(id: string): Promise<any>;
    generateTitle(sessionId: string, firstMessage: string): Promise<any>;

    getMemoryStats(sessionId: string): Promise<any>;
    optimizeMemory(): Promise<any>;
    cleanupMemory(): Promise<any>;

    static version(): string;
}

declare const version: () => string;

export { Config, OfflineIntelligence, version };
