import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/airport/$iata')({
  component: AirportDetail,
})

function AirportDetail() {
  return <div><h1>Airport Detail</h1></div>
}
