import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';

const rootEl = document.getElementById('root');
if (!rootEl) throw new Error('Root element not found');

// Global styles
const style = document.createElement('style');
style.textContent = `
  *, *::before, *::after {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }
  html, body {
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
    background: #0a0a0a;
    color: #e0e0e0;
    -webkit-font-smoothing: antialiased;
    overflow-x: hidden;
  }
  button:active {
    transform: scale(0.97);
  }
  button:hover {
    filter: brightness(1.1);
  }
`;
document.head.appendChild(style);

createRoot(rootEl).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
