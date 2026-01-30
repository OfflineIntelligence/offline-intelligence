package com.offlineintelligence;

/**
 * Result class for search operations
 */
public class SearchResult {
    private int total;
    private String searchType;
    
    public int getTotal() {
        return total;
    }
    
    public void setTotal(int total) {
        this.total = total;
    }
    
    public String getSearchType() {
        return searchType;
    }
    
    public void setSearchType(String searchType) {
        this.searchType = searchType;
    }
    
    @Override
    public String toString() {
        return "SearchResult{total=" + total + ", searchType='" + searchType + "'}";
    }
}