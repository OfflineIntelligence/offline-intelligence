package com.offlineintelligence;

/**
 * Exception class for Offline Intelligence errors
 */
public class OfflineIntelligenceException extends Exception {
    
    public OfflineIntelligenceException(String message) {
        super(message);
    }
    
    public OfflineIntelligenceException(String message, Throwable cause) {
        super(message, cause);
    }
}