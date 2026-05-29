import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { LoginPage } from './pages/LoginPage';
import { ChatPage } from './pages/ChatPage';
import { ProfilePage } from './pages/ProfilePage';
import { FindPage } from './pages/FindPage';
import { HomePage } from './pages/HomePage';
import { QuestionnairePage } from './pages/QuestionnairePage';
import { RecommendationsPage } from './pages/RecommendationsPage';
import { SettingsPage } from './pages/SettingsPage';
import { AdminPage } from './pages/AdminPage';
import { DmPage } from './pages/DmPage';
import { SafetyPage } from './pages/SafetyPage';
import { CompliancePage } from './pages/CompliancePage';
import { AuthGuard } from './components/AuthGuard';
import { OlErrorBoundary } from './components/OlErrorBoundary';
import { OlSpinnerStyle } from './components/OlButton';
import './styles/tokens.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <OlSpinnerStyle />
    <OlErrorBoundary>
      <BrowserRouter>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/" element={<AuthGuard><HomePage /></AuthGuard>} />
          <Route path="/chat" element={<AuthGuard><ChatPage /></AuthGuard>} />
          <Route path="/profile" element={<AuthGuard><ProfilePage /></AuthGuard>} />
          <Route path="/find" element={<AuthGuard><FindPage /></AuthGuard>} />
          <Route path="/questionnaire" element={<AuthGuard><QuestionnairePage /></AuthGuard>} />
          <Route path="/recommendations" element={<AuthGuard><RecommendationsPage /></AuthGuard>} />
          <Route path="/settings" element={<AuthGuard><SettingsPage /></AuthGuard>} />
          <Route path="/admin" element={<AuthGuard><AdminPage /></AuthGuard>} />
          <Route path="/dm" element={<AuthGuard><DmPage /></AuthGuard>} />
          <Route path="/safety" element={<AuthGuard><SafetyPage /></AuthGuard>} />
          <Route path="/compliance" element={<AuthGuard><CompliancePage /></AuthGuard>} />
        </Routes>
      </BrowserRouter>
    </OlErrorBoundary>
  </React.StrictMode>,
);