import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/search')({
  component: Search,
})

function Search() {
  return <div><h1>Search</h1></div>
}
