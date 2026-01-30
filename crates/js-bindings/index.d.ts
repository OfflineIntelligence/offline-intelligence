export interface Message {
  role: string;
  content: string;
}

export interface OptimizationResult {
  optimizedMessages: Message[];
  originalCount: number;
  optimizedCount: number;
  compressionRatio: number;
}

export interface SearchResult {
  results: any[];
  total: number;
  searchType: string;
}

export class Config {
  constructor();
  readonly modelPath: string;
  readonly ctxSize: number;
  readonly batchSize: number;
}

export class OfflineIntelligence {
  constructor();
  
  optimizeContext(sessionId: string, messages: Message[], userQuery?: string | null): Promise<OptimizationResult>;
  search(query: string, sessionId?: string | null, limit?: number): Promise<SearchResult>;
  generateTitle(messages: Message[]): Promise<string>;
}

export { OfflineIntelligence as default };