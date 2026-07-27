package com.example.meliorsonus

import app.cash.sqldelight.db.SqlDriver
import com.example.meliorsonus.db.MeliorSonusDatabase
import com.example.meliorsonus.repository.SavedSheetRepositoryImpl
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.http.headersOf
import io.ktor.utils.io.ByteReadChannel
import kotlinx.coroutines.test.runTest
import kotlin.test.Test

expect fun createDatabaseFactoryTest(): SqlDriver


class SavedSheetRepositoryTest {
    @Test
    fun checkMXL(){
        runTest{
            val driver = createDatabaseFactoryTest()
            val database = MeliorSonusDatabase(driver)
            val repository = SavedSheetRepositoryImpl(database)



            println(repository.getMXL())
        }
    }


    @Test
    fun `checking issue`(){
        val mockEngine = MockEngine { request ->
            respond(
                content = ByteReadChannel("""{"ip":"127.0.0.1"}"""),
                status = HttpStatusCode.OK,
            )
        }
    }
}