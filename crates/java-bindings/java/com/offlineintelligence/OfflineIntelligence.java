package com.offlineintelligence;

public class OfflineIntelligence {
    static {
        System.loadLibrary("offline_intelligence_java");
    }

    private long nativePtr;

    public OfflineIntelligence() {
        this.nativePtr = newInstance();
        if (this.nativePtr == 0) {
            throw new RuntimeException("Failed to create OfflineIntelligence instance");
        }
    }


    public OptimizationResult optimizeContext(String sessionId, Message[] messages, String userQuery) {
        return optimizeContext(nativePtr, sessionId, messages, userQuery);
    }


    public SearchResult search(String query, String sessionId, int limit) {
        return search(nativePtr, query, sessionId, limit);
    }


    public String generateTitle(Message[] messages) {
        return generateTitle(nativePtr, messages);
    }


    public void dispose() {
        if (nativePtr != 0) {
            dispose(nativePtr);
            nativePtr = 0;
        }
    }

    @Override
    protected void finalize() throws Throwable {
        dispose();
        super.finalize();
    }


    private static native long newInstance();
    private static native OptimizationResult optimizeContext(long ptr, String sessionId, Message[] messages, String userQuery);
    private static native SearchResult search(long ptr, String query, String sessionId, int limit);
    private static native String generateTitle(long ptr, Message[] messages);
    private static native void dispose(long ptr);
}
