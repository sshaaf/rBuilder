#include "ecommerce/db/sqlite.hpp"
#include "ecommerce/handlers/health_handler.hpp"
#include "ecommerce/handlers/order_handler.hpp"
#include <cstdio>

int main(int argc, char** argv) {
    sqlite3* db = nullptr;
    if (ecommerce::db::open("ecommerce.db", &db) != 0) {
        std::fprintf(stderr, "failed to open database\n");
        return 1;
    }
    ecommerce::handlers::handle_health(db, nullptr);
    if (argc > 1 && argv[1]) {
        ecommerce::handlers::handle_order(db, argv[1]);
    }
    ecommerce::db::close(db);
    return 0;
}
