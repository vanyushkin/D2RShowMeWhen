import './styles.css';
import { bootstrapApp } from './bootstrap';

bootstrapApp().catch(err => {
  const root = document.querySelector<HTMLDivElement>('#app')!;
  root.innerHTML = `<pre style="padding:1rem;color:#ff9a9a">Bootstrap failed: ${String(err)}</pre>`;
});
