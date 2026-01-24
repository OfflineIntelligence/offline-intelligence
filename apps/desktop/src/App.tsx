import { useState, useEffect } from 'react';
import { ChatWindow } from './components/ChatWindow';
import type { Chat } from './components/ChatWindow';
import { Sidebar } from './components/Sidebar';
import { SearchModal } from './components/SearchModal';
import { fetchConversations } from './api/chat';
import './App.css';

// Profile Modal Component
const ProfileModal: React.FC<{ 
  onClose: () => void;
  onLogout: () => void;
  onDeleteAccount: () => void;
}> = ({ onClose, onLogout, onDeleteAccount }) => {
  // Close modal when clicking outside
  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };
  
  // Handle escape key
  useEffect(() => {
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    
    window.addEventListener('keydown', handleEsc);
    return () => window.removeEventListener('keydown', handleEsc);
  }, [onClose]);
  
  return (
    <div className="profile-overlay" onClick={handleOverlayClick}>
      <div className="profile-modal">
        <div className="profile-header">
          <h2 className="profile-title">User Profile</h2>
          <button className="profile-close" onClick={onClose}>
            <svg width="20" height="20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
        
        <div className="profile-content">
          <div className="profile-section">
            <div className="profile-field">
              <label className="profile-label">Email Address</label>
              <div className="profile-value">user@example.com</div>
            </div>
            
            <div className="profile-field">
              <label className="profile-label">Phone Number</label>
              <div className="profile-value">+1 (555) 123-4567</div>
            </div>
          </div>
          
          <div className="profile-actions">
            <button className="profile-action-button logout" onClick={onLogout}>
              <svg className="action-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
              </svg>
              <span>Logout</span>
            </button>
            
            <button className="profile-action-button delete" onClick={onDeleteAccount}>
              <svg className="action-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
              <span>Delete Account</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

function App() {
  // State management for chat functionality
  const [chats, setChats] = useState<Chat[]>([]);
  const [activeChatId, setActiveChatId] = useState<string | null>(null);
  const [messages, setMessages] = useState<any[]>([]);
  const [chatTitle, setChatTitle] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [isSidebarOpen] = useState(true);
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [isDarkTheme, setIsDarkTheme] = useState(true);
  const [isProfileOpen, setIsProfileOpen] = useState(false);

  // Apply theme to document
  useEffect(() => {
    if (isDarkTheme) {
      document.documentElement.classList.add('dark-theme');
      document.documentElement.classList.remove('light-theme');
    } else {
      document.documentElement.classList.add('light-theme');
      document.documentElement.classList.remove('dark-theme');
    }
  }, [isDarkTheme]);

  // Load saved conversations on startup
  useEffect(() => {
    loadConversations();
  }, []);

  const loadConversations = async () => {
    try {
      const conversations = await fetchConversations();
      const chatList = conversations.map(conv => ({
        id: conv.id,
        title: conv.title,
        messages: [],
        createdAt: new Date(conv.created_at),
        pinned: false
      }));
      setChats(chatList);
    } catch (error) {
      console.error('Failed to load conversations:', error);
    }
  };

  const handleNewChat = () => {
    // Clear current chat state
    setMessages([]);
    setChatTitle(null);
    setSessionId(null);
    setActiveChatId(null);
  };

  const handleSelectChat = async (chatId: string) => {
    // TODO: Load chat messages from backend
    setActiveChatId(chatId);
    // Reset to empty for now
    setMessages([]);
    setChatTitle(chats.find(c => c.id === chatId)?.title || null);
  };

  const handleMessagesUpdate = (newMessages: any[]) => {
    setMessages(newMessages);
  };

  const handleTitleGenerated = (title: string) => {
    setChatTitle(title);
    // Update in chats list
    if (activeChatId) {
      setChats(prev => prev.map(chat => 
        chat.id === activeChatId ? { ...chat, title } : chat
      ));
    }
  };

  const handleSessionIdChange = (newSessionId: string) => {
    setSessionId(newSessionId);
  };

  const handlePinChat = (chatId: string) => {
    setChats(prev => prev.map(chat => 
      chat.id === chatId ? { ...chat, pinned: !chat.pinned } : chat
    ));
  };

  const handleDeleteChat = async (chatId: string) => {
    // TODO: Implement delete functionality
    setChats(prev => prev.filter(chat => chat.id !== chatId));
    if (activeChatId === chatId) {
      handleNewChat();
    }
  };

  const handleOpenSearch = () => {
    setIsSearchOpen(true);
  };

  const handleCloseSearch = () => {
    setIsSearchOpen(false);
  };

  const handleOpenProfile = () => {
    setIsProfileOpen(true);
  };

  const handleCloseProfile = () => {
    setIsProfileOpen(false);
  };

  return (
    <div className="app-container">
      {isSidebarOpen && (
        <Sidebar 
          chats={chats}
          activeChatId={activeChatId}
          onNewChat={handleNewChat}
          onSelectChat={handleSelectChat}
          onOpenSearch={handleOpenSearch}
          onOpenProfile={handleOpenProfile}
          isDarkTheme={isDarkTheme}
          onThemeToggle={() => setIsDarkTheme(!isDarkTheme)}
        />
      )}
      
      <div className="main-content">
        <ChatWindow 
          messages={messages}
          chatTitle={chatTitle}
          chatId={activeChatId}
          sessionId={sessionId}
          onSessionIdChange={handleSessionIdChange}
          onMessagesUpdate={handleMessagesUpdate}
          onTitleGenerated={handleTitleGenerated}
          onPinChat={handlePinChat}
          onDeleteChat={handleDeleteChat}
        />
      </div>

      <SearchModal
        isOpen={isSearchOpen}
        chats={chats}
        onClose={handleCloseSearch}
        onSelectChat={handleSelectChat}
      />
      
      {isProfileOpen && (
        <ProfileModal 
          onClose={handleCloseProfile}
          onLogout={() => console.log('Logout clicked')}
          onDeleteAccount={() => console.log('Delete account clicked')}
        />
      )}
    </div>
  );
}

export default App;