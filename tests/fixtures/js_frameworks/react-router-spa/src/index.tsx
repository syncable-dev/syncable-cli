import { createRoot } from 'react-dom/client';
import { BrowserRouter, Routes, Route } from 'react-router-dom';

const App = () => (
  <BrowserRouter>
    <Routes>
      <Route path="/" element={<h1>React Router SPA Fixture</h1>} />
    </Routes>
  </BrowserRouter>
);

createRoot(document.getElementById('root')!).render(<App />);
