import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/airport/$iata')({
  component: AirportDetail,
})

function AirportDetail() {
  const { iata } = Route.useParams()
  return <div><h1>Airport: {iata}</h1></div>
}
