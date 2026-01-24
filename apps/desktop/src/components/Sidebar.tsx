// Sidebar: Futuristic minimal navigation inspired by Perplexity/Gemini
// Clean glass-morphism design with elevated interactions
import React, { useState } from 'react'
import type { Chat } from './ChatWindow'

interface SidebarProps {
  chats: Chat[];
  activeChatId: string | null;
  onNewChat?: () => void;
  onSelectChat?: (chatId: string) => void;
  onOpenSearch?: () => void;
  onOpenProfile?: () => void;
  isDarkTheme?: boolean;
  onThemeToggle?: () => void;
}

// Settings Modal Component
const SettingsModal: React.FC<{ 
  onClose: () => void; 
  isDarkTheme: boolean; 
  onThemeToggle: () => void;
  onOpenProfile: () => void;
}> = ({ onClose, isDarkTheme, onThemeToggle, onOpenProfile }) => {
  const toggleTheme = () => {
    onThemeToggle();
  };
  
  const handleProfileClick = () => {
    onOpenProfile();
    onClose(); // Close settings modal when opening profile
  };
  
  // Close modal when clicking outside
  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };
  
  // Handle escape key
  React.useEffect(() => {
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    
    window.addEventListener('keydown', handleEsc);
    return () => window.removeEventListener('keydown', handleEsc);
  }, [onClose]);
  
  return (
    <div className="settings-overlay" onClick={handleOverlayClick}>
      <div className="settings-modal">
        <div className="settings-header">
          <h2 className="settings-title">Settings</h2>
          <button className="settings-close" onClick={onClose}>
            <svg width="20" height="20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
        
        <div className="settings-content">
          <div className="settings-section">
            <div className="settings-option" onClick={handleProfileClick}>
              <div className="option-icon">
                <svg width="20" height="20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                </svg>
              </div>
              <div className="option-text">
                <span className="option-label">User Profile</span>
                <span className="option-description">Manage your account settings</span>
              </div>
              <div className="option-arrow">
                <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              </div>
            </div>
            
            <div className="settings-option" onClick={() => console.log('General clicked')}>
              <div className="option-icon">
                <svg width="20" height="20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.548-.99 3.197-.99 4.746 0a1.724 1.724 0 002.573-1.066c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.548-.99 3.197-.99 4.746 0a1.724 1.724 0 002.573-1.066c.426-1.756 2.924-1.756 3.35 0L20 14.5l-4.746 4.746a1.724 1.724 0 00-2.573 1.066c-1.548.99-3.197.99-4.746 0a1.724 1.724 0 00-2.573-1.066L4 14.5l4.746-4.746a1.724 1.724 0 001.066-2.573z" />
                </svg>
              </div>
              <div className="option-text">
                <span className="option-label">General</span>
                <span className="option-description">Application preferences and settings</span>
              </div>
              <div className="option-arrow">
                <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              </div>
            </div>
            
            <div className="settings-option theme-toggle-option" onClick={toggleTheme}>
              <div className="option-icon">
                <svg width="20" height="20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                </svg>
              </div>
              <div className="option-text">
                <span className="option-label">Theme</span>
                <span className="option-description">Switch between light and dark mode</span>
              </div>
              <div className="theme-toggle">
                <div className={`toggle-switch ${isDarkTheme ? 'dark' : 'light'}`}>
                  <div className="toggle-handle"></div>
                </div>
              </div>
            </div>
            
            <div className="settings-option" onClick={() => console.log('About clicked')}>
              <div className="option-icon">
                <svg width="20" height="20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
              <div className="option-text">
                <span className="option-label">About</span>
                <span className="option-description">Learn about Aud.io</span>
              </div>
              <div className="option-arrow">
                <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              </div>
            </div>
            
            <div className="settings-option" onClick={() => console.log('Feedback clicked')}>
              <div className="option-icon">
                <svg width="20" height="20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                </svg>
              </div>
              <div className="option-text">
                <span className="option-label">Feedback</span>
                <span className="option-description">Share your thoughts and suggestions</span>
              </div>
              <div className="option-arrow">
                <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              </div>
            </div>
            
            <div className="settings-option" onClick={() => console.log('Help clicked')}>
              <div className="option-icon">
                <svg width="20" height="20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
              <div className="option-text">
                <span className="option-label">Help</span>
                <span className="option-description">Get assistance and support</span>
              </div>
              <div className="option-arrow">
                <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export const Sidebar: React.FC<SidebarProps> = ({ 
  chats, 
  activeChatId, 
  onNewChat, 
  onSelectChat, 
  onOpenSearch,
  onOpenProfile,
  isDarkTheme = true,
  onThemeToggle = () => {}
}) => {
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  
  const openSettings = () => {
    setIsSettingsOpen(true);
  };
  
  const closeSettings = () => {
    setIsSettingsOpen(false);
  };
  
  return (
    <div className="sidebar-modern">
      {/* Top Actions Section */}
      <div className="sidebar-actions">
        <button className="sidebar-action-button primary" onClick={onNewChat}>
          <span className="action-text">New Chat</span>
        </button>
        
        <button className="sidebar-action-button" onClick={onOpenSearch}>
          <span className="action-text">Chat History</span>
        </button>
      </div>

      {/* Chat History Section */}
      <div className="sidebar-chats-container">
        <div className="chats-header">
          <span className="chats-title">Recent Chats</span>
        </div>
        
        <div className="chats-list">
          {chats
            .sort((a, b) => {
              // Pinned chats first
              if (a.pinned && !b.pinned) return -1;
              if (!a.pinned && b.pinned) return 1;
              // Then by creation date (newest first)
              return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime();
            })
            .map(chat => (
              <button 
                key={chat.id}
                className={`chat-item ${chat.id === activeChatId ? 'active' : ''}`}
                onClick={() => onSelectChat?.(chat.id)}
              >
                <div className="chat-item-content">
                  {chat.pinned && (
                    <svg className="chat-pin-icon" fill="currentColor" viewBox="0 0 24 24">
                      <path d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
                    </svg>
                  )}
                  <span className="chat-title">{chat.title}</span>
                </div>
              </button>
            ))}
        </div>
      </div>

      {/* Settings Button */}
      <button className="sidebar-action-button settings" onClick={openSettings}>
        <span className="action-text">Settings</span>
      </button>
      
      {/* Settings Modal */}
      {isSettingsOpen && (
        <SettingsModal 
          onClose={closeSettings} 
          isDarkTheme={isDarkTheme}
          onThemeToggle={onThemeToggle}
          onOpenProfile={onOpenProfile || (() => {})}
        />
      )}
    </div>
  )
}