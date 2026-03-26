; ModuleID = 'bench_advanced.cpp'
source_filename = "bench_advanced.cpp"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

module asm ".globl _ZSt21ios_base_library_initv"

%"class.std::basic_ostream" = type { ptr, %"class.std::basic_ios" }
%"class.std::basic_ios" = type { %"class.std::ios_base", ptr, i8, i8, ptr, ptr, ptr, ptr }
%"class.std::ios_base" = type { ptr, i64, i64, i32, i32, i32, ptr, %"struct.std::ios_base::_Words", [8 x %"struct.std::ios_base::_Words"], i32, ptr, %"class.std::locale" }
%"struct.std::ios_base::_Words" = type { ptr, i64 }
%"class.std::locale" = type { ptr }
%"class.std::ctype" = type <{ %"class.std::locale::facet.base", [4 x i8], ptr, i8, [7 x i8], ptr, ptr, ptr, i8, [256 x i8], [256 x i8], i8, [6 x i8] }>
%"class.std::locale::facet.base" = type <{ ptr, i32 }>

@_ZSt4cout = external global %"class.std::basic_ostream", align 8
@.str = private unnamed_addr constant [3 x i8] c"  \00", align 1
@.str.1 = private unnamed_addr constant [3 x i8] c": \00", align 1
@.str.2 = private unnamed_addr constant [12 x i8] c"s  (result=\00", align 1
@.str.3 = private unnamed_addr constant [2 x i8] c")\00", align 1
@.str.4 = private unnamed_addr constant [66 x i8] c"=================================================================\00", align 1
@.str.5 = private unnamed_addr constant [61 x i8] c"  Agam vs Python vs Rust vs C++ \E2\80\94 Advanced Benchmark Suite\00", align 1
@.str.6 = private unnamed_addr constant [24 x i8] c"  C++ Runtime (GCC -O3)\00", align 1
@.str.7 = private unnamed_addr constant [10 x i8] c"Sum(100M)\00", align 1
@.str.8 = private unnamed_addr constant [14 x i8] c"Fibonacci(40)\00", align 1
@.str.9 = private unnamed_addr constant [17 x i8] c"PrimeCount(100K)\00", align 1
@.str.10 = private unnamed_addr constant [16 x i8] c"MatMul(100x100)\00", align 1
@.str.11 = private unnamed_addr constant [15 x i8] c"Integrate(10M)\00", align 1
@.str.12 = private unnamed_addr constant [19 x i8] c"  Total C++ time: \00", align 1
@.str.13 = private unnamed_addr constant [2 x i8] c"s\00", align 1

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @_Z8sum_loopx(i64 noundef %0) #0 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %13

3:                                                ; preds = %1
  %4 = add nsw i64 %0, -1
  %5 = zext nneg i64 %4 to i65
  %6 = add nsw i64 %0, -2
  %7 = zext i64 %6 to i65
  %8 = mul i65 %5, %7
  %9 = lshr i65 %8, 1
  %10 = trunc i65 %9 to i64
  %11 = add i64 %10, %0
  %12 = add i64 %11, -1
  br label %13

13:                                               ; preds = %3, %1
  %14 = phi i64 [ 0, %1 ], [ %12, %3 ]
  ret i64 %14
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @_Z9fibonaccix(i64 noundef %0) #0 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %20

3:                                                ; preds = %1
  %4 = and i64 %0, 7
  %5 = icmp ult i64 %0, 8
  br i1 %5, label %8, label %6

6:                                                ; preds = %3
  %7 = and i64 %0, 9223372036854775800
  br label %22

8:                                                ; preds = %22, %3
  %9 = phi i64 [ undef, %3 ], [ %32, %22 ]
  %10 = phi i64 [ 0, %3 ], [ %32, %22 ]
  %11 = phi i64 [ 1, %3 ], [ %33, %22 ]
  %12 = icmp eq i64 %4, 0
  br i1 %12, label %20, label %13

13:                                               ; preds = %8, %13
  %14 = phi i64 [ %15, %13 ], [ %10, %8 ]
  %15 = phi i64 [ %17, %13 ], [ %11, %8 ]
  %16 = phi i64 [ %18, %13 ], [ 0, %8 ]
  %17 = add nsw i64 %14, %15
  %18 = add i64 %16, 1
  %19 = icmp eq i64 %18, %4
  br i1 %19, label %20, label %13, !llvm.loop !5

20:                                               ; preds = %8, %13, %1
  %21 = phi i64 [ 0, %1 ], [ %9, %8 ], [ %15, %13 ]
  ret i64 %21

22:                                               ; preds = %22, %6
  %23 = phi i64 [ 0, %6 ], [ %32, %22 ]
  %24 = phi i64 [ 1, %6 ], [ %33, %22 ]
  %25 = phi i64 [ 0, %6 ], [ %34, %22 ]
  %26 = add nsw i64 %23, %24
  %27 = add nsw i64 %24, %26
  %28 = add nsw i64 %26, %27
  %29 = add nsw i64 %27, %28
  %30 = add nsw i64 %28, %29
  %31 = add nsw i64 %29, %30
  %32 = add nsw i64 %30, %31
  %33 = add nsw i64 %31, %32
  %34 = add i64 %25, 8
  %35 = icmp eq i64 %34, %7
  br i1 %35, label %8, label %22, !llvm.loop !7
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @_Z12count_primesx(i64 noundef %0) #0 {
  %2 = icmp sgt i64 %0, 2
  br i1 %2, label %3, label %7

3:                                                ; preds = %1, %17
  %4 = phi i64 [ %20, %17 ], [ 2, %1 ]
  %5 = phi i64 [ %19, %17 ], [ 0, %1 ]
  %6 = icmp ult i64 %4, 4
  br i1 %6, label %17, label %13

7:                                                ; preds = %17, %1
  %8 = phi i64 [ 0, %1 ], [ %19, %17 ]
  ret i64 %8

9:                                                ; preds = %13
  %10 = add nuw nsw i64 %14, 1
  %11 = mul nsw i64 %10, %10
  %12 = icmp ugt i64 %11, %4
  br i1 %12, label %17, label %13, !llvm.loop !9

13:                                               ; preds = %3, %9
  %14 = phi i64 [ %10, %9 ], [ 2, %3 ]
  %15 = urem i64 %4, %14
  %16 = icmp eq i64 %15, 0
  br i1 %16, label %17, label %9

17:                                               ; preds = %9, %13, %3
  %18 = phi i64 [ 1, %3 ], [ 1, %9 ], [ 0, %13 ]
  %19 = add nuw nsw i64 %5, %18
  %20 = add nuw nsw i64 %4, 1
  %21 = icmp eq i64 %20, %0
  br i1 %21, label %7, label %3, !llvm.loop !10
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @_Z15matrix_multiplyx(i64 noundef %0) #0 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %69

3:                                                ; preds = %1
  %4 = and i64 %0, 3
  %5 = icmp ult i64 %0, 4
  %6 = and i64 %0, 9223372036854775804
  %7 = icmp eq i64 %4, 0
  br label %8

8:                                                ; preds = %3, %66
  %9 = phi i64 [ %67, %66 ], [ 0, %3 ]
  %10 = phi i64 [ %63, %66 ], [ 0, %3 ]
  %11 = mul nsw i64 %9, %0
  br label %12

12:                                               ; preds = %61, %8
  %13 = phi i64 [ 0, %8 ], [ %64, %61 ]
  %14 = phi i64 [ %10, %8 ], [ %63, %61 ]
  br i1 %5, label %45, label %15

15:                                               ; preds = %12, %15
  %16 = phi i64 [ %42, %15 ], [ 0, %12 ]
  %17 = phi i64 [ %41, %15 ], [ 0, %12 ]
  %18 = phi i64 [ %43, %15 ], [ 0, %12 ]
  %19 = add nuw nsw i64 %16, %11
  %20 = mul nsw i64 %16, %0
  %21 = add nuw nsw i64 %20, %13
  %22 = mul nsw i64 %21, %19
  %23 = add nuw nsw i64 %22, %17
  %24 = or disjoint i64 %16, 1
  %25 = add nuw nsw i64 %24, %11
  %26 = mul nsw i64 %24, %0
  %27 = add nuw nsw i64 %26, %13
  %28 = mul nsw i64 %27, %25
  %29 = add nuw nsw i64 %28, %23
  %30 = or disjoint i64 %16, 2
  %31 = add nuw nsw i64 %30, %11
  %32 = mul nsw i64 %30, %0
  %33 = add nuw nsw i64 %32, %13
  %34 = mul nsw i64 %33, %31
  %35 = add nuw nsw i64 %34, %29
  %36 = or disjoint i64 %16, 3
  %37 = add nuw nsw i64 %36, %11
  %38 = mul nsw i64 %36, %0
  %39 = add nuw nsw i64 %38, %13
  %40 = mul nsw i64 %39, %37
  %41 = add nuw nsw i64 %40, %35
  %42 = add nuw nsw i64 %16, 4
  %43 = add i64 %18, 4
  %44 = icmp eq i64 %43, %6
  br i1 %44, label %45, label %15, !llvm.loop !11

45:                                               ; preds = %15, %12
  %46 = phi i64 [ undef, %12 ], [ %41, %15 ]
  %47 = phi i64 [ 0, %12 ], [ %42, %15 ]
  %48 = phi i64 [ 0, %12 ], [ %41, %15 ]
  br i1 %7, label %61, label %49

49:                                               ; preds = %45, %49
  %50 = phi i64 [ %58, %49 ], [ %47, %45 ]
  %51 = phi i64 [ %57, %49 ], [ %48, %45 ]
  %52 = phi i64 [ %59, %49 ], [ 0, %45 ]
  %53 = add nuw nsw i64 %50, %11
  %54 = mul nsw i64 %50, %0
  %55 = add nuw nsw i64 %54, %13
  %56 = mul nsw i64 %55, %53
  %57 = add nuw nsw i64 %56, %51
  %58 = add nuw nsw i64 %50, 1
  %59 = add i64 %52, 1
  %60 = icmp eq i64 %59, %4
  br i1 %60, label %61, label %49, !llvm.loop !12

61:                                               ; preds = %49, %45
  %62 = phi i64 [ %46, %45 ], [ %57, %49 ]
  %63 = add nsw i64 %62, %14
  %64 = add nuw nsw i64 %13, 1
  %65 = icmp eq i64 %64, %0
  br i1 %65, label %66, label %12, !llvm.loop !13

66:                                               ; preds = %61
  %67 = add nuw nsw i64 %9, 1
  %68 = icmp eq i64 %67, %0
  br i1 %68, label %69, label %8, !llvm.loop !14

69:                                               ; preds = %66, %1
  %70 = phi i64 [ 0, %1 ], [ %63, %66 ]
  ret i64 %70
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @_Z12integrate_x2x(i64 noundef %0) #0 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %21

3:                                                ; preds = %1
  %4 = add nsw i64 %0, -1
  %5 = zext nneg i64 %4 to i65
  %6 = add nsw i64 %0, -2
  %7 = zext i64 %6 to i65
  %8 = mul i65 %5, %7
  %9 = add nsw i64 %0, -3
  %10 = zext i64 %9 to i65
  %11 = mul i65 %8, %10
  %12 = lshr i65 %11, 1
  %13 = trunc i65 %12 to i64
  %14 = mul i64 %13, 6148914691236517206
  %15 = add i64 %14, %0
  %16 = lshr i65 %8, 1
  %17 = trunc i65 %16 to i64
  %18 = mul i64 %17, 3
  %19 = add i64 %15, %18
  %20 = add i64 %19, -1
  br label %21

21:                                               ; preds = %3, %1
  %22 = phi i64 [ 0, %1 ], [ %20, %3 ]
  ret i64 %22
}

; Function Attrs: mustprogress uwtable
define dso_local noundef double @_Z9run_benchPKcPFxxEx(ptr noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #1 {
  %4 = tail call i64 @_ZNSt6chrono3_V212system_clock3nowEv() #7
  %5 = tail call noundef i64 %1(i64 noundef %2)
  %6 = tail call i64 @_ZNSt6chrono3_V212system_clock3nowEv() #7
  %7 = sub nsw i64 %6, %4
  %8 = sitofp i64 %7 to double
  %9 = fdiv double %8, 1.000000e+09
  %10 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str, i64 noundef 2)
  %11 = icmp eq ptr %0, null
  br i1 %11, label %12, label %20

12:                                               ; preds = %3
  %13 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !15
  %14 = getelementptr i8, ptr %13, i64 -24
  %15 = load i64, ptr %14, align 8
  %16 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %15
  %17 = getelementptr inbounds %"class.std::ios_base", ptr %16, i64 0, i32 5
  %18 = load i32, ptr %17, align 8, !tbaa !18
  %19 = or i32 %18, 1
  tail call void @_ZNSt9basic_iosIcSt11char_traitsIcEE5clearESt12_Ios_Iostate(ptr noundef nonnull align 8 dereferenceable(264) %16, i32 noundef %19)
  br label %23

20:                                               ; preds = %3
  %21 = tail call noundef i64 @strlen(ptr noundef nonnull dereferenceable(1) %0) #7
  %22 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull %0, i64 noundef %21)
  br label %23

23:                                               ; preds = %12, %20
  %24 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.1, i64 noundef 2)
  %25 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo9_M_insertIdEERSoT_(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, double noundef %9)
  %26 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) %25, ptr noundef nonnull @.str.2, i64 noundef 11)
  %27 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo9_M_insertIxEERSoT_(ptr noundef nonnull align 8 dereferenceable(8) %25, i64 noundef %5)
  %28 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) %27, ptr noundef nonnull @.str.3, i64 noundef 1)
  %29 = load ptr, ptr %27, align 8, !tbaa !15
  %30 = getelementptr i8, ptr %29, i64 -24
  %31 = load i64, ptr %30, align 8
  %32 = getelementptr inbounds i8, ptr %27, i64 %31
  %33 = getelementptr inbounds %"class.std::basic_ios", ptr %32, i64 0, i32 5
  %34 = load ptr, ptr %33, align 8, !tbaa !28
  %35 = icmp eq ptr %34, null
  br i1 %35, label %36, label %37

36:                                               ; preds = %23
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

37:                                               ; preds = %23
  %38 = getelementptr inbounds %"class.std::ctype", ptr %34, i64 0, i32 8
  %39 = load i8, ptr %38, align 8, !tbaa !31
  %40 = icmp eq i8 %39, 0
  br i1 %40, label %44, label %41

41:                                               ; preds = %37
  %42 = getelementptr inbounds %"class.std::ctype", ptr %34, i64 0, i32 9, i64 10
  %43 = load i8, ptr %42, align 1, !tbaa !34
  br label %49

44:                                               ; preds = %37
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %34)
  %45 = load ptr, ptr %34, align 8, !tbaa !15
  %46 = getelementptr inbounds ptr, ptr %45, i64 6
  %47 = load ptr, ptr %46, align 8
  %48 = tail call noundef signext i8 %47(ptr noundef nonnull align 8 dereferenceable(570) %34, i8 noundef signext 10)
  br label %49

49:                                               ; preds = %41, %44
  %50 = phi i8 [ %43, %41 ], [ %48, %44 ]
  %51 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) %27, i8 noundef signext %50)
  %52 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %51)
  ret double %9
}

; Function Attrs: nounwind
declare i64 @_ZNSt6chrono3_V212system_clock3nowEv() local_unnamed_addr #2

; Function Attrs: mustprogress norecurse uwtable
define dso_local noundef i32 @main() local_unnamed_addr #3 {
  %1 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.4, i64 noundef 65)
  %2 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !15
  %3 = getelementptr i8, ptr %2, i64 -24
  %4 = load i64, ptr %3, align 8
  %5 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %4
  %6 = getelementptr inbounds %"class.std::basic_ios", ptr %5, i64 0, i32 5
  %7 = load ptr, ptr %6, align 8, !tbaa !28
  %8 = icmp eq ptr %7, null
  br i1 %8, label %9, label %10

9:                                                ; preds = %0
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

10:                                               ; preds = %0
  %11 = getelementptr inbounds %"class.std::ctype", ptr %7, i64 0, i32 8
  %12 = load i8, ptr %11, align 8, !tbaa !31
  %13 = icmp eq i8 %12, 0
  br i1 %13, label %17, label %14

14:                                               ; preds = %10
  %15 = getelementptr inbounds %"class.std::ctype", ptr %7, i64 0, i32 9, i64 10
  %16 = load i8, ptr %15, align 1, !tbaa !34
  br label %22

17:                                               ; preds = %10
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %7)
  %18 = load ptr, ptr %7, align 8, !tbaa !15
  %19 = getelementptr inbounds ptr, ptr %18, i64 6
  %20 = load ptr, ptr %19, align 8
  %21 = tail call noundef signext i8 %20(ptr noundef nonnull align 8 dereferenceable(570) %7, i8 noundef signext 10)
  br label %22

22:                                               ; preds = %14, %17
  %23 = phi i8 [ %16, %14 ], [ %21, %17 ]
  %24 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %23)
  %25 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %24)
  %26 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.5, i64 noundef 60)
  %27 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !15
  %28 = getelementptr i8, ptr %27, i64 -24
  %29 = load i64, ptr %28, align 8
  %30 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %29
  %31 = getelementptr inbounds %"class.std::basic_ios", ptr %30, i64 0, i32 5
  %32 = load ptr, ptr %31, align 8, !tbaa !28
  %33 = icmp eq ptr %32, null
  br i1 %33, label %34, label %35

34:                                               ; preds = %22
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

35:                                               ; preds = %22
  %36 = getelementptr inbounds %"class.std::ctype", ptr %32, i64 0, i32 8
  %37 = load i8, ptr %36, align 8, !tbaa !31
  %38 = icmp eq i8 %37, 0
  br i1 %38, label %42, label %39

39:                                               ; preds = %35
  %40 = getelementptr inbounds %"class.std::ctype", ptr %32, i64 0, i32 9, i64 10
  %41 = load i8, ptr %40, align 1, !tbaa !34
  br label %47

42:                                               ; preds = %35
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %32)
  %43 = load ptr, ptr %32, align 8, !tbaa !15
  %44 = getelementptr inbounds ptr, ptr %43, i64 6
  %45 = load ptr, ptr %44, align 8
  %46 = tail call noundef signext i8 %45(ptr noundef nonnull align 8 dereferenceable(570) %32, i8 noundef signext 10)
  br label %47

47:                                               ; preds = %39, %42
  %48 = phi i8 [ %41, %39 ], [ %46, %42 ]
  %49 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %48)
  %50 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %49)
  %51 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.6, i64 noundef 23)
  %52 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !15
  %53 = getelementptr i8, ptr %52, i64 -24
  %54 = load i64, ptr %53, align 8
  %55 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %54
  %56 = getelementptr inbounds %"class.std::basic_ios", ptr %55, i64 0, i32 5
  %57 = load ptr, ptr %56, align 8, !tbaa !28
  %58 = icmp eq ptr %57, null
  br i1 %58, label %59, label %60

59:                                               ; preds = %47
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

60:                                               ; preds = %47
  %61 = getelementptr inbounds %"class.std::ctype", ptr %57, i64 0, i32 8
  %62 = load i8, ptr %61, align 8, !tbaa !31
  %63 = icmp eq i8 %62, 0
  br i1 %63, label %67, label %64

64:                                               ; preds = %60
  %65 = getelementptr inbounds %"class.std::ctype", ptr %57, i64 0, i32 9, i64 10
  %66 = load i8, ptr %65, align 1, !tbaa !34
  br label %72

67:                                               ; preds = %60
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %57)
  %68 = load ptr, ptr %57, align 8, !tbaa !15
  %69 = getelementptr inbounds ptr, ptr %68, i64 6
  %70 = load ptr, ptr %69, align 8
  %71 = tail call noundef signext i8 %70(ptr noundef nonnull align 8 dereferenceable(570) %57, i8 noundef signext 10)
  br label %72

72:                                               ; preds = %64, %67
  %73 = phi i8 [ %66, %64 ], [ %71, %67 ]
  %74 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %73)
  %75 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %74)
  %76 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.4, i64 noundef 65)
  %77 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !15
  %78 = getelementptr i8, ptr %77, i64 -24
  %79 = load i64, ptr %78, align 8
  %80 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %79
  %81 = getelementptr inbounds %"class.std::basic_ios", ptr %80, i64 0, i32 5
  %82 = load ptr, ptr %81, align 8, !tbaa !28
  %83 = icmp eq ptr %82, null
  br i1 %83, label %84, label %85

84:                                               ; preds = %72
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

85:                                               ; preds = %72
  %86 = getelementptr inbounds %"class.std::ctype", ptr %82, i64 0, i32 8
  %87 = load i8, ptr %86, align 8, !tbaa !31
  %88 = icmp eq i8 %87, 0
  br i1 %88, label %92, label %89

89:                                               ; preds = %85
  %90 = getelementptr inbounds %"class.std::ctype", ptr %82, i64 0, i32 9, i64 10
  %91 = load i8, ptr %90, align 1, !tbaa !34
  br label %97

92:                                               ; preds = %85
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %82)
  %93 = load ptr, ptr %82, align 8, !tbaa !15
  %94 = getelementptr inbounds ptr, ptr %93, i64 6
  %95 = load ptr, ptr %94, align 8
  %96 = tail call noundef signext i8 %95(ptr noundef nonnull align 8 dereferenceable(570) %82, i8 noundef signext 10)
  br label %97

97:                                               ; preds = %89, %92
  %98 = phi i8 [ %91, %89 ], [ %96, %92 ]
  %99 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %98)
  %100 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %99)
  %101 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !15
  %102 = getelementptr i8, ptr %101, i64 -24
  %103 = load i64, ptr %102, align 8
  %104 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %103
  %105 = getelementptr inbounds %"class.std::basic_ios", ptr %104, i64 0, i32 5
  %106 = load ptr, ptr %105, align 8, !tbaa !28
  %107 = icmp eq ptr %106, null
  br i1 %107, label %108, label %109

108:                                              ; preds = %97
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

109:                                              ; preds = %97
  %110 = getelementptr inbounds %"class.std::ctype", ptr %106, i64 0, i32 8
  %111 = load i8, ptr %110, align 8, !tbaa !31
  %112 = icmp eq i8 %111, 0
  br i1 %112, label %116, label %113

113:                                              ; preds = %109
  %114 = getelementptr inbounds %"class.std::ctype", ptr %106, i64 0, i32 9, i64 10
  %115 = load i8, ptr %114, align 1, !tbaa !34
  br label %121

116:                                              ; preds = %109
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %106)
  %117 = load ptr, ptr %106, align 8, !tbaa !15
  %118 = getelementptr inbounds ptr, ptr %117, i64 6
  %119 = load ptr, ptr %118, align 8
  %120 = tail call noundef signext i8 %119(ptr noundef nonnull align 8 dereferenceable(570) %106, i8 noundef signext 10)
  br label %121

121:                                              ; preds = %113, %116
  %122 = phi i8 [ %115, %113 ], [ %120, %116 ]
  %123 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %122)
  %124 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %123)
  %125 = tail call noundef double @_Z9run_benchPKcPFxxEx(ptr noundef nonnull @.str.7, ptr noundef nonnull @_Z8sum_loopx, i64 noundef 100000000)
  %126 = fadd double %125, 0.000000e+00
  %127 = tail call noundef double @_Z9run_benchPKcPFxxEx(ptr noundef nonnull @.str.8, ptr noundef nonnull @_Z9fibonaccix, i64 noundef 40)
  %128 = fadd double %126, %127
  %129 = tail call noundef double @_Z9run_benchPKcPFxxEx(ptr noundef nonnull @.str.9, ptr noundef nonnull @_Z12count_primesx, i64 noundef 100000)
  %130 = fadd double %128, %129
  %131 = tail call noundef double @_Z9run_benchPKcPFxxEx(ptr noundef nonnull @.str.10, ptr noundef nonnull @_Z15matrix_multiplyx, i64 noundef 100)
  %132 = fadd double %130, %131
  %133 = tail call noundef double @_Z9run_benchPKcPFxxEx(ptr noundef nonnull @.str.11, ptr noundef nonnull @_Z12integrate_x2x, i64 noundef 10000000)
  %134 = fadd double %132, %133
  %135 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !15
  %136 = getelementptr i8, ptr %135, i64 -24
  %137 = load i64, ptr %136, align 8
  %138 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %137
  %139 = getelementptr inbounds %"class.std::basic_ios", ptr %138, i64 0, i32 5
  %140 = load ptr, ptr %139, align 8, !tbaa !28
  %141 = icmp eq ptr %140, null
  br i1 %141, label %142, label %143

142:                                              ; preds = %121
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

143:                                              ; preds = %121
  %144 = getelementptr inbounds %"class.std::ctype", ptr %140, i64 0, i32 8
  %145 = load i8, ptr %144, align 8, !tbaa !31
  %146 = icmp eq i8 %145, 0
  br i1 %146, label %150, label %147

147:                                              ; preds = %143
  %148 = getelementptr inbounds %"class.std::ctype", ptr %140, i64 0, i32 9, i64 10
  %149 = load i8, ptr %148, align 1, !tbaa !34
  br label %155

150:                                              ; preds = %143
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %140)
  %151 = load ptr, ptr %140, align 8, !tbaa !15
  %152 = getelementptr inbounds ptr, ptr %151, i64 6
  %153 = load ptr, ptr %152, align 8
  %154 = tail call noundef signext i8 %153(ptr noundef nonnull align 8 dereferenceable(570) %140, i8 noundef signext 10)
  br label %155

155:                                              ; preds = %147, %150
  %156 = phi i8 [ %149, %147 ], [ %154, %150 ]
  %157 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %156)
  %158 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %157)
  %159 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.12, i64 noundef 18)
  %160 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo9_M_insertIdEERSoT_(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, double noundef %134)
  %161 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) %160, ptr noundef nonnull @.str.13, i64 noundef 1)
  %162 = load ptr, ptr %160, align 8, !tbaa !15
  %163 = getelementptr i8, ptr %162, i64 -24
  %164 = load i64, ptr %163, align 8
  %165 = getelementptr inbounds i8, ptr %160, i64 %164
  %166 = getelementptr inbounds %"class.std::basic_ios", ptr %165, i64 0, i32 5
  %167 = load ptr, ptr %166, align 8, !tbaa !28
  %168 = icmp eq ptr %167, null
  br i1 %168, label %169, label %170

169:                                              ; preds = %155
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

170:                                              ; preds = %155
  %171 = getelementptr inbounds %"class.std::ctype", ptr %167, i64 0, i32 8
  %172 = load i8, ptr %171, align 8, !tbaa !31
  %173 = icmp eq i8 %172, 0
  br i1 %173, label %177, label %174

174:                                              ; preds = %170
  %175 = getelementptr inbounds %"class.std::ctype", ptr %167, i64 0, i32 9, i64 10
  %176 = load i8, ptr %175, align 1, !tbaa !34
  br label %182

177:                                              ; preds = %170
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %167)
  %178 = load ptr, ptr %167, align 8, !tbaa !15
  %179 = getelementptr inbounds ptr, ptr %178, i64 6
  %180 = load ptr, ptr %179, align 8
  %181 = tail call noundef signext i8 %180(ptr noundef nonnull align 8 dereferenceable(570) %167, i8 noundef signext 10)
  br label %182

182:                                              ; preds = %174, %177
  %183 = phi i8 [ %176, %174 ], [ %181, %177 ]
  %184 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) %160, i8 noundef signext %183)
  %185 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %184)
  %186 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.4, i64 noundef 65)
  %187 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !15
  %188 = getelementptr i8, ptr %187, i64 -24
  %189 = load i64, ptr %188, align 8
  %190 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %189
  %191 = getelementptr inbounds %"class.std::basic_ios", ptr %190, i64 0, i32 5
  %192 = load ptr, ptr %191, align 8, !tbaa !28
  %193 = icmp eq ptr %192, null
  br i1 %193, label %194, label %195

194:                                              ; preds = %182
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

195:                                              ; preds = %182
  %196 = getelementptr inbounds %"class.std::ctype", ptr %192, i64 0, i32 8
  %197 = load i8, ptr %196, align 8, !tbaa !31
  %198 = icmp eq i8 %197, 0
  br i1 %198, label %202, label %199

199:                                              ; preds = %195
  %200 = getelementptr inbounds %"class.std::ctype", ptr %192, i64 0, i32 9, i64 10
  %201 = load i8, ptr %200, align 1, !tbaa !34
  br label %207

202:                                              ; preds = %195
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %192)
  %203 = load ptr, ptr %192, align 8, !tbaa !15
  %204 = getelementptr inbounds ptr, ptr %203, i64 6
  %205 = load ptr, ptr %204, align 8
  %206 = tail call noundef signext i8 %205(ptr noundef nonnull align 8 dereferenceable(570) %192, i8 noundef signext 10)
  br label %207

207:                                              ; preds = %199, %202
  %208 = phi i8 [ %201, %199 ], [ %206, %202 ]
  %209 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %208)
  %210 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %209)
  ret i32 0
}

declare noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8), ptr noundef, i64 noundef) local_unnamed_addr #4

declare void @_ZNSt9basic_iosIcSt11char_traitsIcEE5clearESt12_Ios_Iostate(ptr noundef nonnull align 8 dereferenceable(264), i32 noundef) local_unnamed_addr #4

; Function Attrs: mustprogress nofree nounwind willreturn memory(argmem: read)
declare i64 @strlen(ptr nocapture noundef) local_unnamed_addr #5

declare noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo9_M_insertIdEERSoT_(ptr noundef nonnull align 8 dereferenceable(8), double noundef) local_unnamed_addr #4

declare noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo9_M_insertIxEERSoT_(ptr noundef nonnull align 8 dereferenceable(8), i64 noundef) local_unnamed_addr #4

declare noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8), i8 noundef signext) local_unnamed_addr #4

declare noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8)) local_unnamed_addr #4

; Function Attrs: noreturn
declare void @_ZSt16__throw_bad_castv() local_unnamed_addr #6

declare void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570)) local_unnamed_addr #4

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { mustprogress uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { mustprogress norecurse uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #5 = { mustprogress nofree nounwind willreturn memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #6 = { noreturn "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #7 = { nounwind }
attributes #8 = { noreturn }

!llvm.module.flags = !{!0, !1, !2, !3}
!llvm.ident = !{!4}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{i32 7, !"PIE Level", i32 2}
!3 = !{i32 7, !"uwtable", i32 2}
!4 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
!5 = distinct !{!5, !6}
!6 = !{!"llvm.loop.unroll.disable"}
!7 = distinct !{!7, !8}
!8 = !{!"llvm.loop.mustprogress"}
!9 = distinct !{!9, !8}
!10 = distinct !{!10, !8}
!11 = distinct !{!11, !8}
!12 = distinct !{!12, !6}
!13 = distinct !{!13, !8}
!14 = distinct !{!14, !8}
!15 = !{!16, !16, i64 0}
!16 = !{!"vtable pointer", !17, i64 0}
!17 = !{!"Simple C++ TBAA"}
!18 = !{!19, !23, i64 32}
!19 = !{!"_ZTSSt8ios_base", !20, i64 8, !20, i64 16, !22, i64 24, !23, i64 28, !23, i64 32, !24, i64 40, !25, i64 48, !21, i64 64, !26, i64 192, !24, i64 200, !27, i64 208}
!20 = !{!"long", !21, i64 0}
!21 = !{!"omnipotent char", !17, i64 0}
!22 = !{!"_ZTSSt13_Ios_Fmtflags", !21, i64 0}
!23 = !{!"_ZTSSt12_Ios_Iostate", !21, i64 0}
!24 = !{!"any pointer", !21, i64 0}
!25 = !{!"_ZTSNSt8ios_base6_WordsE", !24, i64 0, !20, i64 8}
!26 = !{!"int", !21, i64 0}
!27 = !{!"_ZTSSt6locale", !24, i64 0}
!28 = !{!29, !24, i64 240}
!29 = !{!"_ZTSSt9basic_iosIcSt11char_traitsIcEE", !19, i64 0, !24, i64 216, !21, i64 224, !30, i64 225, !24, i64 232, !24, i64 240, !24, i64 248, !24, i64 256}
!30 = !{!"bool", !21, i64 0}
!31 = !{!32, !21, i64 56}
!32 = !{!"_ZTSSt5ctypeIcE", !33, i64 0, !24, i64 16, !30, i64 24, !24, i64 32, !24, i64 40, !24, i64 48, !21, i64 56, !21, i64 57, !21, i64 313, !21, i64 569}
!33 = !{!"_ZTSNSt6locale5facetE", !26, i64 8}
!34 = !{!21, !21, i64 0}
