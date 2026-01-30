const { OfflineIntelligence: NativeOfflineIntelligence, Config: NativeConfig } = require('./offline_intelligence_js.node');

class Message {
  constructor(role, content) {
    this.role = role;
    this.content = content;
  }
}

class Config extends NativeConfig {
  constructor() {
    super();
  }
}

class OfflineIntelligence extends NativeOfflineIntelligence {
  constructor() {
    super();
  }

  async optimizeContext(sessionId, messages, userQuery = null) {
    const result = await super.optimize_context(sessionId, messages, userQuery);
    return {
      optimizedMessages: result.optimized_messages || [],
      originalCount: result.original_count || messages.length,
      optimizedCount: result.optimized_count || 0,
      compressionRatio: result.compression_ratio || 0.0
    };
  }

  async search(query, sessionId = null, limit = 10) {
    const result = await super.search(query, sessionId, limit);
    return {
      results: result.results || [],
      total: result.total || 0,
      searchType: result.search_type || 'keyword'
    };
  }

  async generateTitle(messages) {
    return await super.generate_title(messages);
  }
}

module.exports = {
  OfflineIntelligence,
  Config,
  Message
};