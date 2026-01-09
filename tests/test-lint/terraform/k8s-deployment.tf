# Test Terraform file with Kubernetes resources

resource "kubernetes_deployment" "nginx" {
  metadata {
    name      = "nginx-deployment"
    namespace = "default"
  }

  spec {
    replicas = 3

    selector {
      match_labels = {
        app = "nginx"
      }
    }

    template {
      metadata {
        labels = {
          app = "nginx"
        }
      }

      spec {
        container {
          name  = "nginx"
          image = "nginx:1.21"

          resources {
            requests {
              cpu    = "100m"
              memory = "128Mi"
            }
            limits {
              cpu    = "500m"
              memory = "512Mi"
            }
          }
        }
      }
    }
  }
}

# Over-provisioned deployment - should trigger warnings
resource "kubernetes_deployment_v1" "over_provisioned" {
  metadata {
    name = "over-provisioned-app"
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app = "over-provisioned"
      }
    }

    template {
      metadata {
        labels = {
          app = "over-provisioned"
        }
      }

      spec {
        container {
          name  = "app"
          image = "myapp:latest"

          resources {
            requests {
              cpu    = "4000m"
              memory = "8Gi"
            }
            limits {
              cpu    = "8000m"
              memory = "16Gi"
            }
          }
        }
      }
    }
  }
}

# Missing resources - should trigger warnings
resource "kubernetes_deployment" "no_resources" {
  metadata {
    name = "no-resources-app"
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app = "no-resources"
      }
    }

    template {
      metadata {
        labels = {
          app = "no-resources"
        }
      }

      spec {
        container {
          name  = "app"
          image = "myapp:v2"
          # No resources defined - should be flagged
        }
      }
    }
  }
}
