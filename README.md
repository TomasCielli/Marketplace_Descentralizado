<p align="center">
<img align="center" src="https://media.tenor.com/3d8r8wIlXGEAAAAj/duck-pato.gif"/> 
</p>

# Trabajo Práctico Final – Marketplace Descentralizado tipo MercadoLibre

**Materia:** Seminario de Lenguajes – Opción Rust  
**Tecnología:** Rust + Ink! + Substrate  
**Cobertura de tests requerida:** ≥ 85%  
**Entregas:**  
- ⭕ Primera entrega obligatoria: **18 de julio**  
- ✅ Entrega final completa: **Antes de finalizar 2025**

---

## 📜 Introducción

El presente trabajo práctico final tiene como objetivo integrar los conocimientos adquiridos durante el cursado de la materia **Seminario de Lenguajes – Opción Rust**, aplicando conceptos de programación en Rust orientados al desarrollo de contratos inteligentes sobre la plataforma **Substrate** utilizando el framework **Ink!**.

La consigna propone desarrollar una **plataforma descentralizada de compra-venta de productos**, inspirada en modelos como MercadoLibre, pero ejecutada completamente en un entorno blockchain. El sistema deberá dividirse en **dos contratos inteligentes**: uno encargado de gestionar la lógica principal del marketplace y otro destinado a la generación de reportes a partir de los datos públicos del primero.

El proyecto busca que el estudiante no solo practique la sintaxis y semántica de Rust, sino que también comprenda el diseño modular de contratos inteligentes, la separación de responsabilidades, la validación de roles y permisos, y la importancia de la transparencia, trazabilidad y reputación en contextos descentralizados.

Se espera que las entregas incluyan una implementación funcional, correctamente testeada, documentada y con una cobertura de pruebas mínima del 85%.

---

## Contrato 1 – `MarketplacePrincipal` (Core funcional + reputación)

### Funcionalidades

#### 👤 Registro y gestión de usuarios
- Registro de usuario con rol: `Comprador`, `Vendedor` o ambos.
- Posibilidad de modificar roles posteriores.

#### 📦 Publicación de productos
- Publicar producto con nombre, descripción, precio, cantidad y categoría.
- Solo disponible para usuarios con rol `Vendedor`.
- Visualización de productos propios.

#### 🛒 Compra y órdenes
- Crear orden de compra (solo `Compradores`).
- Al comprar: se crea la orden y se descuenta stock.
- Estados de orden: `pendiente`, `enviado`, `recibido`, `cancelada`.
- Solo el `Vendedor` puede marcar como `enviado`.
- Solo el `Comprador` puede marcar como `recibido` o `cancelada` si aún está `pendiente`.
- Cancelación requiere consentimiento mutuo.

#### ⭐ Reputación bidireccional
- Cuando la orden esté `recibida`, ambas partes pueden calificar:
  - El `Comprador` califica al `Vendedor`.
  - El `Vendedor` califica al `Comprador`.
- Calificación: entero del 1 al 5.
- Solo una calificación por parte y por orden.
- Reputación acumulada pública:
  - `reputacion_como_comprador`
  - `reputacion_como_vendedor`

---

## Contrato 2 – `ReportesView` (solo lectura)

### Funcionalidades
- Consultar top 5 vendedores con mejor reputación.
- Consultar top 5 compradores con mejor reputación.
- Ver productos más vendidos.
- Estadísticas por categoría: total de ventas, calificación promedio.
- Cantidad de órdenes por usuario.

**Nota:** este contrato solo puede leer datos del contrato 1. No puede emitir calificaciones, modificar órdenes ni publicar productos.

---

## 📊 Requisitos generales

- ✅ Cobertura de tests ≥ 85% entre ambos contratos.
- ✅ Tests deben contemplar:
  - Flujos completos de compra y calificación.
  - Validaciones y errores esperados.
  - Permisos por rol.
- ✅ Código comentado y bien estructurado.


---

## 🔺 Entrega Mínima – 18 de julio

Incluye:
- Contrato 1 con:
  - Registro de usuarios.
  - Publicación de productos.
  - Compra de productos.
  - Gestión básica de órdenes (`pendiente`, `enviado`, `recibido`).
  - Todo documentado segun lo visto en clase de como documentar en Rust
  - Tests con cobertura ≥ 85%.
  - Address del contrato desplegado en Shibuya Testnet.


---

## 🌟 Entrega Final – Fin de año

Incluye:
- Toda la funcionalidad de ambos contratos.
- Reputación completa bidireccional.
- Reportes por lectura (contrato 2).
- Tests con cobertura ≥ 85%.
- Documentación técnica clara.

### Bonus (hasta +20%):
- Sistema de disputas.
- Simulación de pagos.


😊😊




