package com.offlineintelligence;

public class OptimizationResult {
    private int originalCount;
    private int optimizedCount;
    private float compressionRatio;

    public int getOriginalCount() {
        return originalCount;
    }

    public void setOriginalCount(int originalCount) {
        this.originalCount = originalCount;
    }

    public int getOptimizedCount() {
        return optimizedCount;
    }

    public void setOptimizedCount(int optimizedCount) {
        this.optimizedCount = optimizedCount;
    }

    public float getCompressionRatio() {
        return compressionRatio;
    }

    public void setCompressionRatio(float compressionRatio) {
        this.compressionRatio = compressionRatio;
    }

    @Override
    public String toString() {
        return "OptimizationResult{originalCount=" + originalCount +
               ", optimizedCount=" + optimizedCount +
               ", compressionRatio=" + compressionRatio + "}";
    }
}
