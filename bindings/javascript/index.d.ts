declare class Config {
    modelPath: string;
    llamaBin: string;
    llamaHost: string;
    llamaPort: number;
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
    constructor(config: Config);
    
    healthCheck(): Promise<any>;
    generateStream(prompt: string, options?: any): Promise<any>;
    getStatus(): Promise<any>;
    loadModel(modelPath: string): Promise<any>;
    
    static version(): string;
}

declare const version: () => string;

export { Config, OfflineIntelligence, version };