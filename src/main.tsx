import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'

console.log('=== main.tsx starting ===')

try {
  const root = document.getElementById('root')
  console.log('Root element:', root)
  
  if (!root) {
    console.error('Root element not found!')
    document.body.innerHTML = '<h1 style="color:red">Error: Root element not found</h1>'
  } else {
    ReactDOM.createRoot(root).render(
      <React.StrictMode>
        <App />
      </React.StrictMode>,
    )
    console.log('=== React rendered successfully ===')
  }
} catch (e) {
  console.error('=== React render error ===', e)
  document.body.innerHTML = '<h1 style="color:red">React Error: ' + (e as Error).message + '</h1>'
}
